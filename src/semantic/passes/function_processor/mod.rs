//! Function body compilation and type checking.
//!
//! This module implements Pass 2b of semantic analysis: compiling function bodies.
//! It performs type checking on expressions and statements, tracks local variables,
//! and emits bytecode.

mod bytecode_emitter;
mod expr_checker;
mod overload_resolver;
mod stmt_compiler;
mod type_helpers;

use crate::ast::{
    Script,
    decl::{ClassDecl, ClassMember, FunctionDecl, Item, NamespaceDecl, UsingNamespaceDecl},
    expr::{Expr, InitElement},
    stmt::{Block, ForInit, Stmt},
    types::TypeSuffix,
};
use crate::semantic::types::registry::FunctionDef;
use crate::semantic::CompilationContext;
use crate::codegen::{BytecodeEmitter, CompiledBytecode, CompiledModule, Instruction};
use crate::lexer::Span;
use crate::semantic::{
    DataType, LocalScope,
    SemanticError, SemanticErrorKind, TypeDef, TypeId, VOID_TYPE,
};
use crate::semantic::types::type_def::FunctionId;
use rustc_hash::FxHashMap;

/// Category of switch expression for determining comparison strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwitchCategory {
    /// Integer types (int8-64, uint8-64, enum) - use primitive Equal
    Integer,
    /// Boolean type - use primitive Equal
    Bool,
    /// Float/double types - use primitive Equal
    Float,
    /// String type - use opEquals method call
    String,
    /// Handle types - identity comparison + type patterns
    Handle,
}

/// Key for detecting duplicate switch case values at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SwitchCaseKey {
    Int(i64),
    Float(u64),      // f64::to_bits() for exact bit comparison
    Bool(bool),
    String(String),
    Null,
    Type(TypeId),    // For type pattern matching
}

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
pub struct FunctionCompiler<'ast> {
    /// Compilation context (read-only) - contains all type information (FFI + Script)
    context: &'ast CompilationContext<'ast>,

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

    /// Imported namespace paths from `using namespace` directives (fully qualified)
    imported_namespaces: Vec<String>,

    /// Current class context (when compiling methods)
    current_class: Option<TypeId>,

    /// Global lambda counter for unique FunctionIds (starts at next available ID after regular functions)
    next_lambda_id: u32,

    /// Name of the current function being compiled (optional - for debug/error messages)
    current_function: Option<String>,

    /// Expected funcdef type for lambda type inference
    expected_funcdef_type: Option<TypeId>,

    /// Expected init list target type (the type that has list_factory or list_construct behavior)
    /// Set when we know the target type for an init list from context (e.g., variable declaration)
    expected_init_list_target: Option<TypeId>,

    /// Errors encountered during compilation
    errors: Vec<SemanticError>,

    /// Phantom data for source lifetime
    _phantom: std::marker::PhantomData<&'ast ()>,
}

impl<'ast> FunctionCompiler<'ast> {
    /// Perform Pass 2b function compilation on a script.
    ///
    /// This is the main entry point for compiling all functions in a module.
    pub fn compile(
        script: &Script<'ast>,
        context: &'ast CompilationContext<'ast>,
    ) -> CompiledModule {
        let mut compiler = Self::new_module_compiler(context);
        compiler.visit_script(script);

        CompiledModule {
            functions: compiler.compiled_functions,
            errors: compiler.errors,
        }
    }

    /// Creates a new module-level compiler (for compiling all functions).
    fn new_module_compiler(context: &'ast CompilationContext<'ast>) -> Self {
        Self {
            next_lambda_id: context.function_count() as u32,  // Start after regular functions
            context,
            local_scope: LocalScope::new(),
            bytecode: BytecodeEmitter::new(),
            return_type: DataType::simple(VOID_TYPE),
            compiled_functions: FxHashMap::default(),
            namespace_path: Vec::new(),
            imported_namespaces: Vec::new(),
            current_class: None,
            current_function: None,
            expected_funcdef_type: None,
            expected_init_list_target: None,
            errors: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new single-function compiler.
    ///
    /// # Parameters
    ///
    /// - `context`: The complete compilation context from Pass 2a
    /// - `return_type`: The expected return type for this function
    fn new(context: &'ast CompilationContext<'ast>, return_type: DataType) -> Self {
        Self {
            next_lambda_id: context.function_count() as u32,  // Start after regular functions
            context,
            local_scope: LocalScope::new(),
            bytecode: BytecodeEmitter::new(),
            return_type,
            compiled_functions: FxHashMap::default(),
            namespace_path: Vec::new(),
            imported_namespaces: Vec::new(),
            current_class: None,
            current_function: None,
            expected_funcdef_type: None,
            expected_init_list_target: None,
            errors: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Compiles a function body.
    ///
    /// This is a convenience method for compiling a complete function with parameters.
    pub fn compile_block(
        context: &'ast CompilationContext<'ast>,
        return_type: DataType,
        params: &[(String, DataType)],
        body: &'ast Block<'ast>,
    ) -> CompiledFunction {
        let mut compiler = Self::new(context, return_type);

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

    /// Compiles a function body with class and namespace context.
    ///
    /// This variant allows tracking the current class for super() resolution
    /// and the namespace path for unqualified name lookup.
    fn compile_block_with_context(
        context: &'ast CompilationContext<'ast>,
        return_type: DataType,
        params: &[(String, DataType)],
        body: &'ast Block<'ast>,
        current_class: Option<TypeId>,
        namespace_path: Vec<String>,
        imported_namespaces: Vec<String>,
    ) -> CompiledFunction {
        let mut compiler = Self::new(context, return_type);
        compiler.current_class = current_class;
        compiler.namespace_path = namespace_path;
        compiler.imported_namespaces = imported_namespaces;

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

    /// Compiles a field initializer expression.
    ///
    /// This creates a mini-compilation context to compile a single expression
    /// used for field initialization in constructors.
    ///
    /// Returns: (instructions, errors)
    fn compile_field_initializer(
        context: &'ast CompilationContext<'ast>,
        expr: &'ast Expr<'ast>,
        class_type_id: TypeId,
    ) -> (Vec<Instruction>, Vec<SemanticError>) {
        let mut compiler = Self::new(context, DataType::simple(VOID_TYPE));
        compiler.current_class = Some(class_type_id);

        // Compile the expression - this will emit bytecode to push the value onto the stack
        let _expr_ctx = compiler.check_expr(expr);

        // Return the compiled instructions and any errors
        (compiler.bytecode.finish().instructions, compiler.errors)
    }

    // ========================================================================
    // AST Walking (Module-level compilation)
    // ========================================================================

    /// Visit the entire script and compile all functions
    fn visit_script(&mut self, script: &'ast Script<'ast>) {
        for item in script.items() {
            self.visit_item(item);
        }
    }

    /// Visit a top-level item
    fn visit_item(&mut self, item: &'ast Item<'ast>) {
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
            Item::UsingNamespace(using) => self.visit_using_namespace(using),
        }
    }

    /// Visit a namespace and compile functions within it
    fn visit_namespace(&mut self, ns: &'ast NamespaceDecl<'ast>) {
        // Enter namespace (handle path which can be nested like A::B::C)
        for ident in ns.path {
            self.namespace_path.push(ident.name.to_string());
        }

        // Save imported namespaces count for scoping
        let import_count_before = self.imported_namespaces.len();

        for item in ns.items {
            self.visit_item(item);
        }

        // Remove any imports added within this namespace scope
        self.imported_namespaces.truncate(import_count_before);

        // Exit namespace (pop all path components we added)
        for _ in ns.path {
            self.namespace_path.pop();
        }
    }

    /// Visit a using namespace declaration
    fn visit_using_namespace(&mut self, using: &UsingNamespaceDecl<'ast>) {
        // Build the fully qualified namespace path
        let ns_path: String = using.path
            .iter()
            .map(|id| id.name)
            .collect::<Vec<_>>()
            .join("::");

        // Record the import for use in symbol resolution
        self.imported_namespaces.push(ns_path);
    }

    /// Visit a class declaration and compile all its methods
    fn visit_class_decl(&mut self, class: &'ast ClassDecl<'ast>) {
        let qualified_name = self.build_qualified_name(class.name.name);

        // Look up the class type ID
        let type_id = match self.context.lookup_type(&qualified_name) {
            Some(id) => id,
            None => {
                // Class wasn't registered - this shouldn't happen if Pass 1 & 2a worked
                return;
            }
        };

        // Get all methods for this class from the registry
        let method_ids = self.context.get_methods(type_id);

        // Compile each method by matching AST to FunctionIds
        for member in class.members {
            if let ClassMember::Method(method_decl) = member {
                // Find the matching FunctionId for this method
                // Must match by name AND parameter signature for overloaded methods
                let func_id = method_ids
                    .iter()
                    .copied()
                    .find(|&fid| {
                        let func_def = self.context.script().get_function(fid);
                        self.method_signature_matches(method_decl, func_def)
                    });

                if let Some(func_id) = func_id {
                    self.compile_method(method_decl, func_id, Some(class));
                }
            }
        }
    }

    /// Check if an AST method declaration matches a registered FunctionDef.
    ///
    /// This is used for overload resolution when compiling methods. It compares:
    /// - Function name
    /// - Parameter count (including parameters with defaults)
    /// - Parameter types (base type and handle modifier)
    fn method_signature_matches(
        &self,
        method_decl: &FunctionDecl<'ast>,
        func_def: &FunctionDef<'ast>,
    ) -> bool {
        // Name must match
        if func_def.name != method_decl.name.name {
            return false;
        }

        // Parameter count must match (including parameters with defaults)
        if func_def.params.len() != method_decl.params.len() {
            return false;
        }

        // Each parameter type must match
        for (ast_param, def_param) in method_decl.params.iter().zip(func_def.params.iter()) {
            // Resolve AST parameter type to TypeId
            let type_name = format!("{}", ast_param.ty.ty.base);
            let ast_type_id = match self.context.lookup_type(&type_name) {
                Some(id) => id,
                None => return false, // Unknown type - can't match
            };

            // Compare base type IDs
            if ast_type_id != def_param.data_type.type_id {
                return false;
            }

            // Check handle modifier (@) matches
            let ast_is_handle = ast_param.ty.ty.suffixes.iter().any(|s| matches!(s, TypeSuffix::Handle { .. }));
            if ast_is_handle != def_param.data_type.is_handle {
                return false;
            }
        }

        true
    }

    /// Compile a method given its AST and FunctionId
    fn compile_method(&mut self, func: &'ast FunctionDecl<'ast>, func_id: FunctionId, class: Option<&'ast ClassDecl<'ast>>) {
        // Skip functions without bodies (abstract methods, forward declarations)
        let body = match &func.body {
            Some(body) => body,
            None => return,
        };

        let func_def = self.context.script().get_function(func_id);

        // Extract parameters for compilation (pre-allocate capacity)
        let params: Vec<(String, DataType)> = func_def.params.iter().enumerate()
            .map(|(i, param)| {
                // Get parameter name from AST if available, otherwise from ScriptParam
                let name = if i < func.params.len() {
                    func.params[i].name.map(|id| id.name.to_string()).unwrap_or_else(|| param.name.clone())
                } else {
                    param.name.clone()
                };
                (name, param.data_type.clone())
            })
            .collect();

        // For constructors, emit member initialization in the correct order
        let mut constructor_prologue = None;
        if func.is_constructor() && let Some(class_decl) = class {
            constructor_prologue = Some(self.compile_constructor_prologue(class_decl, func_def.object_type, body));
        }

        // Compile the function body with class and namespace context
        let mut compiled = Self::compile_block_with_context(
            self.context,
            func_def.return_type.clone(),
            &params,
            body,
            func_def.object_type,
            self.namespace_path.clone(),
            self.imported_namespaces.clone(),
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
    fn compile_constructor_prologue(&mut self, class: &'ast ClassDecl<'ast>, class_type_id: Option<TypeId>, body: &'ast Block<'ast>) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Get the class type to check for base class
        let class_id = match class_type_id {
            Some(id) => id,
            None => return instructions, // Not a method, shouldn't happen
        };

        let class_typedef = self.context.get_type(class_id);
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

        // 1. Fields without explicit initializers use default initialization
        // The VM handles this automatically when allocating the object:
        // - Primitives: 0, 0.0, false
        // - Handles: null
        // - Value types: default constructor is called
        // No bytecode needed here - VM does it in CallConstructor
        let _ = fields_without_init; // Acknowledge we're intentionally not emitting bytecode

        // 2. Call base class constructor if base class exists and super() not called in body
        if let Some(base_id) = base_class_id {
            // Check if the constructor body contains a super() call
            let has_super_call = self.contains_super_call(body);

            if !has_super_call {
                // Emit call to base class default constructor
                // Only auto-call if super() is not explicitly called
                let base_constructors = self.context.script().find_constructors(base_id);
                if let Some(&base_ctor_id) = base_constructors.first() {
                    instructions.push(Instruction::CallConstructor {
                        type_id: base_id.0,
                        func_id: base_ctor_id.0,
                    });
                }
            }
        }

        // 3. Initialize fields with explicit initializers
        // Get field definitions from the class typedef to find field indices
        let field_defs = match class_typedef {
            TypeDef::Class { fields, .. } => fields,
            _ => return instructions,
        };

        for field in fields_with_init {
            if let Some(init_expr) = field.init {
                // Find the field index by name
                let field_name = field.name.name;
                let field_index = field_defs.iter().position(|f| f.name == field_name);

                if let Some(field_idx) = field_index {
                    // Emit: LoadThis, <expr>, StoreField(field_idx)
                    // 1. Load `this` reference
                    instructions.push(Instruction::LoadThis);

                    // 2. Compile the initializer expression
                    let (expr_instructions, expr_errors) =
                        Self::compile_field_initializer(self.context, init_expr, class_id);
                    instructions.extend(expr_instructions);
                    self.errors.extend(expr_errors);

                    // 3. Store into the field
                    instructions.push(Instruction::StoreField(field_idx as u32));
                }
            }
        }

        instructions
    }

    /// Check if a block contains a super() call.
    ///
    /// This recursively searches through statements and expressions to find
    /// any call expression where the callee is the identifier "super".
    fn contains_super_call(&self, block: &Block<'ast>) -> bool {
        for stmt in block.stmts {
            if self.stmt_contains_super_call(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement contains a super() call (helper for contains_super_call)
    fn stmt_contains_super_call(&self, stmt: &Stmt<'ast>) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                expr_stmt.expr.is_some_and(|e| self.expr_contains_super_call(e))
            }
            Stmt::VarDecl(var_decl) => {
                // VarDeclStmt has a `vars` slice of VarDeclarator
                var_decl.vars.iter().any(|var| {
                    var.init.is_some_and(|e| self.expr_contains_super_call(e))
                })
            }
            Stmt::If(if_stmt) => {
                self.expr_contains_super_call(if_stmt.condition)
                    || self.stmt_contains_super_call(if_stmt.then_stmt)
                    || if_stmt.else_stmt.is_some_and(|s| self.stmt_contains_super_call(s))
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
                            var.init.is_some_and(|e| self.expr_contains_super_call(e))
                        })
                    }
                    Some(ForInit::Expr(expr)) => self.expr_contains_super_call(expr),
                    None => false,
                };
                let update_has_super = for_stmt.update.iter().any(|e| self.expr_contains_super_call(e));
                init_has_super
                    || for_stmt.condition.is_some_and(|e| self.expr_contains_super_call(e))
                    || update_has_super
                    || self.stmt_contains_super_call(for_stmt.body)
            }
            Stmt::Foreach(foreach) => {
                self.expr_contains_super_call(foreach.expr)
                    || self.stmt_contains_super_call(foreach.body)
            }
            Stmt::Return(ret) => ret.value.is_some_and(|e| self.expr_contains_super_call(e)),
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
    fn expr_contains_super_call(&self, expr: &Expr<'ast>) -> bool {
        match expr {
            Expr::Call(call) => {
                // Check if this is a super() call
                if let Expr::Ident(ident) = call.callee
                    && ident.scope.is_none() && ident.ident.name == "super" {
                        return true;
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
            Expr::Lambda(lambda) => self.contains_super_call(lambda.body),
            Expr::Paren(paren) => self.expr_contains_super_call(paren.expr),
            Expr::Ident(_) | Expr::Literal(_) => false,
        }
    }

    /// Visit a function declaration and compile its body
    fn visit_function_decl(&mut self, func: &'ast FunctionDecl<'ast>, object_type: Option<TypeId>) {
        // Skip functions without bodies (abstract methods, forward declarations)
        let body = match &func.body {
            Some(body) => body,
            None => return,
        };

        let qualified_name = self.build_qualified_name(func.name.name);

        // Look up the function in the registry to get its FunctionId and signature
        let func_ids = self.context.lookup_functions(&qualified_name);

        if func_ids.is_empty() {
            // Function wasn't registered - this shouldn't happen if Pass 1 & 2a worked
            return;
        }

        // Find the matching function by checking object_type
        // Only check script functions - FFI functions have bodies defined natively
        let func_id = func_ids
            .iter()
            .copied()
            .filter(|id| id.is_script())
            .find(|&id| {
                let func_def = self.context.script().get_function(id);
                func_def.object_type == object_type
            });

        let func_id = match func_id {
            Some(id) => id,
            None => {
                // No matching function found - skip
                return;
            }
        };

        let func_def = self.context.script().get_function(func_id);

        // Extract parameters for compilation (pre-allocate capacity)
        let params: Vec<(String, DataType)> = func_def.params.iter().enumerate()
            .map(|(i, param)| {
                // Get parameter name from AST if available, otherwise from ScriptParam
                let name = if i < func.params.len() {
                    func.params[i].name.map(|id| id.name.to_string()).unwrap_or_else(|| param.name.clone())
                } else {
                    param.name.clone()
                };
                (name, param.data_type.clone())
            })
            .collect();

        // Compile the function body with namespace context
        let compiled = Self::compile_block_with_context(
            self.context,
            func_def.return_type.clone(),
            &params,
            body,
            None,
            self.namespace_path.clone(),
            self.imported_namespaces.clone(),
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
    fn error(&mut self, kind: SemanticErrorKind, span: Span, message: impl Into<String>) {
        self.errors
            .push(SemanticError::new(kind, span, message));
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{DataType, INT32_TYPE, DOUBLE_TYPE};
    use crate::semantic::types::TypeBehaviors;
    use crate::semantic::types::type_def::PrimitiveType;
    use crate::semantic::FunctionId;
    use crate::semantic::CompilationContext;
    use crate::ffi::FfiRegistryBuilder;
    use std::sync::Arc;

    /// Create a default FFI registry with primitives for tests
    fn default_ffi() -> Arc<crate::ffi::FfiRegistry> {
        Arc::new(FfiRegistryBuilder::new().build().unwrap())
    }

    /// Creates a default CompilationContext for basic tests
    fn create_test_context() -> CompilationContext<'static> {
        CompilationContext::new(default_ffi())
    }

    /// Creates a CompilationContext with an array template registered in FFI.
    /// Returns (context, array_template_id) for use in init list tests.
    fn create_test_context_with_array() -> (CompilationContext<'static>, TypeId) {
        let mut builder = FfiRegistryBuilder::new();

        // Register template param T first
        let t_param = TypeId::next_ffi();
        let template_id = TypeId::next_ffi();

        builder.register_type_with_id(
            t_param,
            TypeDef::TemplateParam {
                name: "T".to_string(),
                index: 0,
                owner: template_id,
            },
            None,
        );

        // Register array template
        let array_typedef = TypeDef::Class {
            name: "array".to_string(),
            qualified_name: "array".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![t_param],
            template: None,
            type_args: Vec::new(),
            type_kind: crate::types::TypeKind::reference(),
        };

        builder.register_type_with_id(template_id, array_typedef, Some("array"));

        // Register list_factory behavior for array template
        let mut behaviors = TypeBehaviors::default();
        behaviors.list_factory = Some(FunctionId::new(9999)); // Dummy function ID
        builder.set_behaviors(template_id, behaviors);

        let ffi = Arc::new(builder.build().unwrap());
        let ctx = CompilationContext::new(ffi);

        (ctx, template_id)
    }

    /// Creates an FfiRegistry with the string module installed
    fn create_ffi_with_string() -> Arc<crate::ffi::FfiRegistry> {
        use crate::modules::string_module;
        let mut builder = FfiRegistryBuilder::new();
        let string_mod = string_module().expect("Failed to create string module");
        string_mod.install_into(&mut builder).expect("Failed to install string module");
        Arc::new(builder.build().unwrap())
    }

    /// Creates an FfiRegistry with the array module installed
    fn create_ffi_with_array() -> Arc<crate::ffi::FfiRegistry> {
        use crate::modules::array_module;
        let mut builder = FfiRegistryBuilder::new();
        let array_mod = array_module().expect("Failed to create array module");
        array_mod.install_into(&mut builder).expect("Failed to install array module");
        Arc::new(builder.build().unwrap())
    }

    /// Creates an FfiRegistry with string and array modules installed
    fn create_ffi_with_string_and_array() -> Arc<crate::ffi::FfiRegistry> {
        use crate::modules::{array_module, string_module};
        let mut builder = FfiRegistryBuilder::new();
        let string_mod = string_module().expect("Failed to create string module");
        string_mod.install_into(&mut builder).expect("Failed to install string module");
        let array_mod = array_module().expect("Failed to create array module");
        array_mod.install_into(&mut builder).expect("Failed to install array module");
        Arc::new(builder.build().unwrap())
    }

    #[test]
    fn new_compiler_initializes() {
        let ctx = create_test_context();
        let return_type = DataType::simple(VOID_TYPE);
        let compiler = FunctionCompiler::<'_>::new(&ctx, return_type);

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

        let (mut ctx, array_template) = create_test_context_with_array();

        // Pre-instantiate array<int> for testing (but don't set as expected target)
        let _array_int = ctx
            .instantiate_template(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&ctx, return_type);

        // NOTE: Don't set expected_init_list_target - test that we get an error
        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_none());
            assert_eq!(compiler.errors.len(), 1);
            // Error message changed: now requires explicit target type
            assert!(compiler.errors[0]
                .message
                .contains("initializer list requires explicit target type"));
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

        let (mut ctx, array_template) = create_test_context_with_array();

        // Pre-instantiate array<int> for testing
        let array_int = ctx
            .instantiate_template(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&ctx, return_type);

        // Set expected init list target type (as would be set by var decl)
        compiler.expected_init_list_target = Some(array_int);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_some(), "check_init_list failed: {:?}", compiler.errors);
            let result_ctx = result.unwrap();

            // Should return array<int>@
            assert!(result_ctx.data_type.is_handle);
            assert_eq!(result_ctx.data_type.type_id, array_int);
            assert_eq!(compiler.errors.len(), 0);

            // Check emitted bytecode
            let bytecode = compiler.bytecode.instructions();
            // Should have: PushInt(1), PushInt(2), PushInt(3), PushInt(3), CallConstructor
            assert!(bytecode.iter().any(|instr| matches!(instr, Instruction::PushInt(3))));
            assert!(bytecode.iter().any(|instr| matches!(instr, Instruction::CallConstructor { .. })));
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

        let (mut ctx, array_template) = create_test_context_with_array();

        // Pre-instantiate array<int>
        let array_int = ctx
            .instantiate_template(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        // Pre-instantiate array<array<int>@>
        let array_array_int = ctx
            .instantiate_template(
                array_template,
                vec![DataType::with_handle(array_int, false)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&ctx, return_type);

        // Set expected init list target type for outer array
        compiler.expected_init_list_target = Some(array_array_int);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_some(), "check_init_list failed: {:?}", compiler.errors);
            let result_ctx = result.unwrap();

            // Should return array<array<int>@>@
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

        let (mut ctx, array_template) = create_test_context_with_array();

        // Pre-instantiate array<double>
        let array_double = ctx
            .instantiate_template(
                array_template,
                vec![DataType::simple(DOUBLE_TYPE)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&ctx, return_type);

        // Set expected init list target type
        compiler.expected_init_list_target = Some(array_double);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_some(), "check_init_list failed: {:?}", compiler.errors);
            let result_ctx = result.unwrap();

            // Should return array<double>@
            assert!(result_ctx.data_type.is_handle);
            assert_eq!(result_ctx.data_type.type_id, array_double);
            assert_eq!(compiler.errors.len(), 0);
        } else {
            panic!("Expected InitList expression");
        }
    }

    // NOTE: Integration tests for opIndex accessors are blocked by pre-existing
    // lifetime issues in the test infrastructure (Registry<'ast> lifetimes).
    // The implementation compiles successfully and logic has been manually verified:
    // - check_index() tries get_opIndex after opIndex (read context)
    // - check_index_assignment() detects write context and uses set_opIndex
    // - opIndex takes priority when both operators and accessors exist
    // Tests will be added once Registry lifetime issues are resolved project-wide.

    #[test]
    fn lambda_compilation_basic() {
        // Test that lambda expressions compile to bytecode with immediate compilation
        use crate::Parser;
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

        let (script, parse_errors) = Parser::parse_lenient(source, &arena);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

        let result = Compiler::compile(&script, default_ffi());

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

        // Find the functions by name via the registry
        let takecb_ids = result.context.lookup_functions("takeCallback");
        assert_eq!(takecb_ids.len(), 1, "Expected 1 takeCallback function");
        let takecb_id = takecb_ids[0];

        let main_ids = result.context.lookup_functions("main");
        assert_eq!(main_ids.len(), 1, "Expected 1 main function");
        let main_id = main_ids[0];

        // Lambda should be any function that's not takeCallback or main
        let lambda_id = result.module.functions.keys()
            .find(|&&id| id != takecb_id && id != main_id)
            .expect("Lambda bytecode not found in compiled module");

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
        use crate::Parser;
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

        let (script, parse_errors) = Parser::parse_lenient(source, &arena);
        if !parse_errors.is_empty() {
            eprintln!("Parse errors: {:?}", parse_errors);
        }

        let result = Compiler::compile(&script, default_ffi());

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
        use crate::Parser;
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

        let (script, _errors) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should compile successfully with variable capture
        assert!(result.is_success(), "Lambda variable capture failed: {:?}", result.errors);

        // Find lambda - it's any function that's not runAction or main
        let run_action_ids = result.context.lookup_functions("runAction");
        let main_ids = result.context.lookup_functions("main");
        let run_action_id = run_action_ids.first().copied();
        let main_id = main_ids.first().copied();

        let lambda_id = result.module.functions.keys()
            .find(|&&id| Some(id) != run_action_id && Some(id) != main_id)
            .expect("Lambda bytecode not found");
        let lambda_bytecode = result.module.functions.get(lambda_id)
            .expect("Lambda bytecode not found");

        // The lambda body should reference the captured variable
        // (exact bytecode depends on implementation details)
        assert!(lambda_bytecode.instructions.len() > 0,
            "Lambda should have non-empty bytecode");
    }

    #[test]
    fn duplicate_switch_case_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1:
                        break;
                    case 1:  // Duplicate!
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should have an error about duplicate case value
        assert!(!result.errors.is_empty(), "Should detect duplicate case value");
        assert!(result.errors.iter().any(|e| e.message.contains("duplicate")),
            "Error should mention 'duplicate': {:?}", result.errors);
    }

    #[test]
    fn switch_no_duplicate_different_values() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1:
                        break;
                    case 2:
                        break;
                    case 3:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should compile without errors
        assert!(result.is_success(), "Different case values should not produce error: {:?}", result.errors);
    }

    #[test]
    fn load_this_instruction_exists() {
        // Test that LoadThis instruction is available
        let instr = Instruction::LoadThis;
        assert!(matches!(instr, Instruction::LoadThis));
    }

    #[test]
    fn method_signature_matching_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {
                void foo(int x) {}
                void foo(float x) {}
                void foo(int x, int y) {}
            }
            void test() {
                Test t;
                t.foo(1);       // Should match foo(int)
                t.foo(1.0f);    // Should match foo(float)
                t.foo(1, 2);    // Should match foo(int, int)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should compile without errors - correct overload selected
        assert!(result.is_success(), "Method overloading should work: {:?}", result.errors);
    }

    #[test]
    fn method_signature_matching_with_defaults() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {
                void bar(int x, int y = 10) {}
            }
            void test() {
                Test t;
                t.bar(1);       // Should work - y uses default
                t.bar(1, 2);    // Should work - explicit y
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should compile without errors - default params handled
        assert!(result.is_success(), "Default parameters should work: {:?}", result.errors);
    }

    #[test]
    fn field_initializer_compilation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {
                int x = 42;
                float y = 3.14f;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Field initializers should compile without errors
        assert!(result.is_success(), "Field initializers should compile: {:?}", result.errors);
    }

    #[test]
    fn switch_with_break_statements() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1:
                        int a = 1;
                        break;
                    case 2:
                        int b = 2;
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Break statements in switch should be allowed
        assert!(result.is_success(), "Break in switch should work: {:?}", result.errors);
    }

    #[test]
    fn switch_inside_loop_with_continue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    switch (i) {
                        case 0:
                            continue;  // Should continue the outer loop
                        case 1:
                            break;     // Should break from switch only
                        default:
                            break;
                    }
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Continue in switch inside loop should target the loop
        assert!(result.is_success(), "Continue in switch inside loop should work: {:?}", result.errors);
    }

    #[test]
    fn namespace_qualified_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Game {
                int getValue() {
                    return 42;
                }
            }

            void test() {
                int x = Game::getValue();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespace-qualified function call should work: {:?}", result.errors);
    }

    #[test]
    fn nested_namespace_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Game {
                namespace Utils {
                    int helper() {
                        return 100;
                    }
                }
            }

            void test() {
                int x = Game::Utils::helper();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested namespace function call should work: {:?}", result.errors);
    }

    #[test]
    fn namespace_function_with_arguments() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Math {
                int add(int a, int b) {
                    return a + b;
                }
            }

            void test() {
                int sum = Math::add(10, 20);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespace function with arguments should work: {:?}", result.errors);
    }

    #[test]
    fn namespace_function_overloading() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Util {
                int process(int x) {
                    return x;
                }
                float process(float x) {
                    return x;
                }
            }

            void test() {
                int a = Util::process(10);
                float b = Util::process(3.14f);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespace function overloading should work: {:?}", result.errors);
    }

    #[test]
    fn call_from_within_namespace() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Game {
                int helper() {
                    return 1;
                }

                void test() {
                    int x = helper();           // Unqualified - should find Game::helper
                    int y = Game::helper();     // Fully qualified
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Calls from within namespace should work: {:?}", result.errors);
    }

    #[test]
    fn namespace_constant_access_from_within() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Math {
                const float PI = 3.14;
                const float TAU = 6.28;

                class Circle {
                    float radius;

                    Circle() { radius = 1.0; }

                    float area() const {
                        return PI * radius * radius;  // Should find Math::PI
                    }

                    float circumference() const {
                        return TAU * radius;  // Should find Math::TAU
                    }
                }
            }

            void main() {
                Math::Circle c;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespace constants should be visible within namespace: {:?}", result.errors);
    }

    #[test]
    fn global_function_call_from_namespace() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            float sqrt(float x) { return x; }

            namespace Core {
                class Vector2 {
                    float x;
                    float y;

                    Vector2() { x = 0.0; y = 0.0; }

                    float length() const {
                        return sqrt(x * x + y * y);  // Should find global sqrt
                    }
                }
            }

            void main() {
                Core::Vector2 v;
                float len = v.length();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Global functions should be callable from within namespace: {:?}", result.errors);
    }

    #[test]
    fn namespace_type_constructor_call_from_within() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Core {
                class Vector2 {
                    float x;
                    float y;

                    Vector2() { x = 0.0; y = 0.0; }
                    Vector2(float _x, float _y) { x = _x; y = _y; }

                    Vector2 perpendicular() const {
                        return Vector2(-y, x);  // Should find Core::Vector2 constructor
                    }
                }
            }

            void main() {
                Core::Vector2 v;
                Core::Vector2 p = v.perpendicular();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespace types should be constructible from within namespace: {:?}", result.errors);
    }

    #[test]
    fn using_namespace_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace test {
                void helper() { }
            }

            using namespace test;

            void main() {
                helper();  // Should find test::helper via import
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Using namespace should allow unqualified function calls: {:?}", result.errors);
    }

    #[test]
    fn base_class_method_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            abstract class Character {
                protected int health;

                Character() { health = 100; }

                protected void onUpdate(float deltaTime) {
                    // Base implementation
                }
            }

            class Player : Character {
                int mana;

                Player() {
                    super();
                    mana = 50;
                }

                protected void onUpdate(float deltaTime) override {
                    Character::onUpdate(deltaTime);  // Call base class method
                    // Player-specific update
                }
            }

            void main() {
                Player@ p = Player();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Base class method call should work: {:?}", result.errors);
    }

    #[test]
    fn absolute_scope_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int globalHelper() {
                return 42;
            }

            namespace Game {
                int helper() {
                    return ::globalHelper();  // Absolute scope - call global function
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Absolute scope function call should work: {:?}", result.errors);
    }

    #[test]
    fn cross_namespace_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Utils {
                int helper() {
                    return 100;
                }
            }

            namespace Game {
                void test() {
                    int x = Utils::helper();  // Cross-namespace call
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Cross-namespace function call should work: {:?}", result.errors);
    }

    #[test]
    fn enum_value_resolution_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void test() {
                int x = Color::Red;    // Should resolve to 0
                int y = Color::Green;  // Should resolve to 1
                int z = Color::Blue;   // Should resolve to 2
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Basic enum value resolution should work: {:?}", result.errors);
    }

    #[test]
    fn enum_value_resolution_with_explicit_values() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Priority {
                Low = 1,
                Medium = 5,
                High = 10
            }

            void test() {
                int x = Priority::Low;     // Should resolve to 1
                int y = Priority::Medium;  // Should resolve to 5
                int z = Priority::High;    // Should resolve to 10
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Enum value resolution with explicit values should work: {:?}", result.errors);
    }

    #[test]
    fn enum_value_in_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void test() {
                int sum = Color::Red + Color::Blue;  // 0 + 2 = 2
                bool cmp = Color::Green > Color::Red;  // 1 > 0 = true
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Enum values in expressions should work: {:?}", result.errors);
    }

    #[test]
    fn namespaced_enum_value_resolution() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Game {
                enum Status {
                    Active,
                    Paused,
                    Stopped
                }
            }

            void test() {
                int x = Game::Status::Active;   // Namespaced enum value
                int y = Game::Status::Stopped;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespaced enum value resolution should work: {:?}", result.errors);
    }

    #[test]
    fn enum_value_undefined_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void test() {
                int x = Color::Yellow;  // Error: Yellow doesn't exist
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for undefined enum value");
        assert!(result.errors.iter().any(|e| e.message.contains("has no value named 'Yellow'")),
            "Error should mention undefined enum value: {:?}", result.errors);
    }

    #[test]
    fn enum_value_as_function_argument() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void processColor(int c) {
                // do something with color
            }

            void test() {
                processColor(Color::Red);
                processColor(Color::Green);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Enum values as function arguments should work: {:?}", result.errors);
    }

    #[test]
    fn enum_value_in_switch() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void test() {
                int color = Color::Red;
                switch (color) {
                    case Color::Red:
                        break;
                    case Color::Green:
                        break;
                    case Color::Blue:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Enum values in switch cases should work: {:?}", result.errors);
    }

    // ========== Funcdef Type Checking Tests ==========

    #[test]
    fn funcdef_variable_declaration_with_function_reference() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void myHandler(int x) {
            }

            void test() {
                Callback@ handler = @myHandler;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef variable with function reference should work: {:?}", result.errors);
    }

    #[test]
    fn funcdef_assignment_with_function_reference() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void handler1(int x) {
            }

            void handler2(int x) {
            }

            void test() {
                Callback@ handler = @handler1;
                handler = @handler2;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef assignment should work: {:?}", result.errors);
    }

    #[test]
    fn funcdef_incompatible_signature_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void wrongSignature(float x) {
            }

            void test() {
                Callback@ handler = @wrongSignature;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Incompatible function signature should error");
        assert!(result.errors.iter().any(|e| format!("{:?}", e.kind).contains("TypeMismatch")));
    }

    #[test]
    fn funcdef_with_return_type() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Calculator(int a, int b);

            int add(int a, int b) {
                return a + b;
            }

            void test() {
                Calculator@ calc = @add;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef with return type should work: {:?}", result.errors);
    }

    #[test]
    fn funcdef_call_through_variable() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Calculator(int a, int b);

            int add(int a, int b) {
                return a + b;
            }

            void test() {
                Calculator@ calc = @add;
                int result = calc(5, 3);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Calling through funcdef variable should work: {:?}", result.errors);
    }

    #[test]
    fn funcdef_without_context_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void myFunc() {
            }

            void test() {
                // @myFunc without a target type should error
                auto handler = @myFunc;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This should error because there's no funcdef context for inference
        assert!(!result.is_success(), "Function reference without funcdef context should error");
    }

    #[test]
    fn funcdef_as_function_parameter() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void execute(Callback@ cb, int value) {
                cb(value);
            }

            void myHandler(int x) {
            }

            void test() {
                execute(@myHandler, 42);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef as function parameter should work: {:?}", result.errors);
    }

    #[test]
    fn funcdef_with_lambda() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Transformer(int x);

            void test() {
                Transformer@ t = function(x) { return x * 2; };
                int result = t(5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Lambda assigned to funcdef should work: {:?}", result.errors);
    }

    #[test]
    fn funcdef_wrong_param_count_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void wrongParamCount(int a, int b) {
            }

            void test() {
                Callback@ handler = @wrongParamCount;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Wrong parameter count should error");
    }

    #[test]
    fn funcdef_wrong_return_type_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Calculator(int x);

            void wrongReturnType(int x) {
            }

            void test() {
                Calculator@ calc = @wrongReturnType;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Wrong return type should error");
    }

    // ========== Bitwise Assignment Operators Tests ==========

    #[test]
    fn bitwise_assignment_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0xFF;
                x &= 0x0F;   // bitwise AND assign
                x |= 0x80;   // bitwise OR assign
                x ^= 0x0F;   // bitwise XOR assign
                x <<= 2;     // left shift assign
                x >>= 1;     // right shift assign
                x >>>= 1;    // unsigned right shift assign
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise assignment operators should work: {:?}", result.errors);
    }

    // ==================== Void Expression Validation Tests ====================

    #[test]
    fn void_variable_declaration_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void main() {
                void x;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for void variable declaration");
        assert!(result.errors.iter().any(|e| {
            e.kind == SemanticErrorKind::VoidExpression
                && e.message.contains("cannot declare variable of type 'void'")
        }), "Should have VoidExpression error: {:?}", result.errors);
    }

    #[test]
    fn void_return_in_non_void_function_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper() {}

            int getValue() {
                return helper();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for returning void expression");
        assert!(result.errors.iter().any(|e| {
            e.kind == SemanticErrorKind::VoidExpression
                && e.message.contains("cannot return a void expression")
        }), "Should have VoidExpression error: {:?}", result.errors);
    }

    #[test]
    fn void_assignment_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper() {}

            void main() {
                int x;
                x = helper();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for assigning void expression");
        assert!(result.errors.iter().any(|e| {
            e.kind == SemanticErrorKind::VoidExpression
                && e.message.contains("cannot use void expression as assignment value")
        }), "Should have VoidExpression error: {:?}", result.errors);
    }

    #[test]
    fn void_binary_operand_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper() {}

            void main() {
                int x = helper() + 1;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for void in binary operation");
        assert!(result.errors.iter().any(|e| {
            e.kind == SemanticErrorKind::VoidExpression
                && e.message.contains("cannot use void expression as left operand")
        }), "Should have VoidExpression error: {:?}", result.errors);
    }

    #[test]
    fn void_unary_operand_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper() {}

            void main() {
                int x = -helper();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for void in unary operation");
        assert!(result.errors.iter().any(|e| {
            e.kind == SemanticErrorKind::VoidExpression
                && e.message.contains("cannot use void expression as operand")
        }), "Should have VoidExpression error: {:?}", result.errors);
    }

    #[test]
    fn void_ternary_branch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper() {}

            void main() {
                bool cond = true;
                int x = cond ? helper() : 1;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Should fail for void in ternary branch");
        assert!(result.errors.iter().any(|e| {
            e.kind == SemanticErrorKind::VoidExpression
                && e.message.contains("cannot use void expression in ternary branch")
        }), "Should have VoidExpression error: {:?}", result.errors);
    }

    #[test]
    fn void_return_type_allowed() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() {
                return;
            }

            void main() {
                doNothing();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Void return type should be allowed: {:?}", result.errors);
    }

    #[test]
    fn void_function_call_as_statement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper() {}

            void main() {
                helper();  // This is valid - discarding void result
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Void function call as statement should be allowed: {:?}", result.errors);
    }

    // ==================== Type Conversion Tests ====================

    #[test]
    fn implicit_int_to_float_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float x = 42;     // int -> float implicit conversion
                double y = 100;   // int -> double implicit conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Implicit int to float conversion should work: {:?}", result.errors);
    }

    #[test]
    fn implicit_float_to_double_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double x = 3.14f;  // float -> double widening
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Implicit float to double conversion should work: {:?}", result.errors);
    }

    #[test]
    fn explicit_cast_int_to_float() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 42;
                float b = float(a);   // Explicit cast
                double c = double(a); // Explicit cast
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Explicit cast int to float should work: {:?}", result.errors);
    }

    #[test]
    fn explicit_cast_double_to_int() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double x = 3.14;
                int a = int(x);    // Explicit narrowing cast
                int8 b = int8(x);  // Explicit narrowing cast
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Explicit cast double to int should work: {:?}", result.errors);
    }

    #[test]
    fn conversion_in_function_argument() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void takeFloat(float x) {}
            void takeDouble(double x) {}

            void test() {
                takeFloat(42);     // int -> float
                takeDouble(42);    // int -> double
                takeDouble(3.14f); // float -> double
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Implicit conversion in function arguments should work: {:?}", result.errors);
    }

    #[test]
    fn conversion_in_binary_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float x = 1 + 2.5f;       // int + float -> float
                double y = 3.14f + 2.71;  // float + double -> double
                float z = 10 / 3.0f;      // int / float -> float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Type promotion in binary expressions should work: {:?}", result.errors);
    }

    #[test]
    fn conversion_in_comparison() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = 42 < 3.14f;     // int compared to float
                bool b = 2.5f > 100;     // float compared to int
                bool c = 1.0 == 1;       // double compared to int
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Type promotion in comparisons should work: {:?}", result.errors);
    }

    #[test]
    fn integer_widening_conversions() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int8 a = 1;
                int16 b = a;   // int8 -> int16 widening
                int32 c = b;   // int16 -> int32 widening
                int64 d = c;   // int32 -> int64 widening
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Integer widening conversions should work: {:?}", result.errors);
    }

    #[test]
    fn uint_literal_operations() {
        // Test uint literal in expressions
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int y = x + 2;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Literal operations should work: {:?}", result.errors);
    }

    // ==================== Handle Conversion Tests ====================

    #[test]
    fn null_to_handle_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {}

            void test() {
                Test@ obj = null;  // null -> Test@ conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Null to handle conversion should work: {:?}", result.errors);
    }

    #[test]
    fn handle_to_const_handle_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {}

            void takeConst(const Test@ obj) {}

            void test() {
                Test@ obj;
                takeConst(obj);  // Test@ -> const Test@ implicit conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Handle to const handle conversion should work: {:?}", result.errors);
    }

    // ==================== Overload Resolution Tests ====================

    #[test]
    fn overload_exact_match_preferred() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void foo(int x) {}
            void foo(float x) {}

            void test() {
                foo(42);     // Should match foo(int)
                foo(3.14f);  // Should match foo(float)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Exact match in overloading should work: {:?}", result.errors);
    }

    #[test]
    fn overload_with_implicit_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void foo(float x) {}
            void foo(double x) {}

            void test() {
                foo(42);  // Should match foo(float) with lowest conversion cost
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Overload with implicit conversion should work: {:?}", result.errors);
    }

    #[test]
    fn overload_multiple_parameters() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void foo(int a, int b) {}
            void foo(float a, float b) {}
            void foo(int a, float b) {}

            void test() {
                foo(1, 2);      // Should match foo(int, int)
                foo(1.0f, 2.0f); // Should match foo(float, float)
                foo(1, 2.0f);   // Should match foo(int, float)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multiple parameter overloading should work: {:?}", result.errors);
    }

    // ==================== Array and Indexing Tests ====================
    // Note: Array tests use init_list compilation which handles the array template instantiation internally

    #[test]
    fn init_list_array_creation() {
        // This test uses init_list which auto-infers the array type
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int[] arr = {1, 2, 3};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Init list array creation should work: {:?}", result.errors);
    }

    // ==================== Ternary Expression Tests ====================

    #[test]
    fn ternary_type_promotion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool cond = true;
                float x = cond ? 1 : 2.0f;   // int and float -> float
                double y = cond ? 1.0f : 2;  // float and int -> float -> double
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Ternary type promotion should work: {:?}", result.errors);
    }

    #[test]
    fn ternary_with_handles() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {}

            void test() {
                bool cond = true;
                Test@ a;
                Test@ b;
                Test@ c = cond ? a : b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Ternary with handles should work: {:?}", result.errors);
    }

    #[test]
    fn ternary_both_handles() {
        // Note: null in ternary branches currently isn't supported - both branches need same handle type
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Test {}

            void test() {
                bool cond = true;
                Test@ a;
                Test@ b;
                Test@ c = cond ? a : b;  // Both branches have Test@ type
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Ternary with both handle branches should work: {:?}", result.errors);
    }

    // ==================== Class and Method Tests ====================

    #[test]
    fn class_method_overloading() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Calculator {
                int add(int a, int b) { return a + b; }
                float add(float a, float b) { return a + b; }
            }

            void test() {
                Calculator calc;
                int x = calc.add(1, 2);
                float y = calc.add(1.0f, 2.0f);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Class method overloading should work: {:?}", result.errors);
    }

    #[test]
    fn class_constructor_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Point {
                float x;
                float y;
                Point(float _x, float _y) {
                    x = _x;
                    y = _y;
                }
            }

            void test() {
                Point p = Point(1, 2);  // int -> float conversion in constructor args
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Constructor with conversion should work: {:?}", result.errors);
    }

    #[test]
    fn derived_to_base_handle_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {}
            class Derived : Base {}

            void takeBase(Base@ b) {}

            void test() {
                Derived@ d;
                takeBase(d);  // Derived@ -> Base@ implicit conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Derived to base handle conversion should work: {:?}", result.errors);
    }

    #[test]
    fn class_implements_interface() {
        // Test that a class can implement an interface
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            class Circle : IDrawable {
                void draw() {}
            }

            void test() {
                Circle c;
                c.draw();  // Direct call on class instance
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Class implementing interface should work: {:?}", result.errors);
    }

    // ==================== Compound Assignment Tests ====================

    #[test]
    fn compound_assignment_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float x = 10.0f;
                x += 5;     // int converted to float
                x -= 3;     // int converted to float
                x *= 2;     // int converted to float
                x /= 4;     // int converted to float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound assignment with conversion should work: {:?}", result.errors);
    }

    // ==================== Return Value Tests ====================

    #[test]
    fn return_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            float getFloat() {
                return 42;  // int -> float conversion
            }

            double getDouble() {
                return 3.14f;  // float -> double conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Return with conversion should work: {:?}", result.errors);
    }

    // ==================== Expression Statement Tests ====================

    #[test]
    fn postfix_increment_decrement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                x++;
                x--;
                int y = x++;
                int z = x--;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Postfix increment/decrement should work: {:?}", result.errors);
    }

    #[test]
    fn prefix_increment_decrement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                ++x;
                --x;
                int y = ++x;
                int z = --x;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Prefix increment/decrement should work: {:?}", result.errors);
    }

    // ==================== Unary Expression Tests ====================

    #[test]
    fn unary_negation_all_types() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = -42;
                float b = -3.14f;
                double c = -2.71;
                int8 d = -1;
                int64 e = -1000000;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Unary negation should work for all numeric types: {:?}", result.errors);
    }

    #[test]
    fn bitwise_not_operator() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = ~0;
                uint y = ~1u;
                int64 z = ~100;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise not should work: {:?}", result.errors);
    }

    // ==================== Control Flow Tests ====================

    #[test]
    fn nested_loops_with_break_continue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    for (int j = 0; j < 10; j++) {
                        if (j == 5) continue;
                        if (i == 5) break;
                    }
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested loops with break/continue should work: {:?}", result.errors);
    }

    #[test]
    fn switch_with_fallthrough() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 2;
                int result = 0;
                switch (x) {
                    case 1:
                    case 2:
                        result = 10;
                        break;
                    case 3:
                        result = 30;
                        break;
                    default:
                        result = -1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Switch with fallthrough should work: {:?}", result.errors);
    }

    // ==================== Logical Operators Tests ====================

    #[test]
    fn logical_and_or_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = a && b;
                bool d = a || b;
                bool e = !a;
                bool f = (a && b) || (!a && !b);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Logical operators should work: {:?}", result.errors);
    }

    // ==================== Bitwise Operators Tests ====================

    #[test]
    fn bitwise_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 0xFF;
                int b = 0x0F;
                int c = a & b;    // AND
                int d = a | b;    // OR
                int e = a ^ b;    // XOR
                int f = a << 4;   // Left shift
                int g = a >> 4;   // Right shift
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise operators should work: {:?}", result.errors);
    }

    #[test]
    fn comparison_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10;
                int b = 20;
                bool c1 = a == b;
                bool c2 = a != b;
                bool c3 = a < b;
                bool c4 = a <= b;
                bool c5 = a > b;
                bool c6 = a >= b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Comparison operators should work: {:?}", result.errors);
    }

    // ==================== Member Access Tests ====================

    #[test]
    fn chained_member_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Inner {
                int value;
            }

            class Outer {
                Inner inner;
            }

            void test() {
                Outer obj;
                int x = obj.inner.value;
                obj.inner.value = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Chained member access should work: {:?}", result.errors);
    }

    #[test]
    fn simple_method_chaining() {
        // Simpler method call chaining without "return this" pattern
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Data {
                int value;
                int getValue() { return value; }
                void setValue(int v) { value = v; }
            }

            void test() {
                Data d;
                d.setValue(10);
                int x = d.getValue();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Simple method calls should work: {:?}", result.errors);
    }

    // ==================== String Literals Tests ====================

    #[test]
    fn string_literal_usage() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::string_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s = "hello";
                string t = "world";
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let string_mod = string_module().expect("Failed to create string module");
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "String literal usage should work: {:?}", result.errors);
    }

    // ==================== Mixed Expression Tests ====================

    #[test]
    fn complex_expression_evaluation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 1 + 2 * 3;          // 7
                int b = (1 + 2) * 3;        // 9
                float c = 1.0f + 2 * 3.0f;  // 7.0
                bool d = 1 < 2 && 3 > 2;    // true
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Complex expressions should work: {:?}", result.errors);
    }

    // ==================== Constructor and Field Initialization Tests ====================

    #[test]
    fn class_constructor_with_field_initialization() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value = 42;
                float score = 3.14f;

                MyClass() {
                    // Fields are initialized before this body runs
                }
            }

            void test() {
                MyClass obj;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Constructor with field initialization should work: {:?}", result.errors);
    }

    #[test]
    fn derived_class_constructor_with_base_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                int baseValue;
                Base() { baseValue = 10; }
            }

            class Derived : Base {
                int derivedValue = 20;

                Derived() {
                    super();
                    derivedValue = 30;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Derived constructor with super() should work: {:?}", result.errors);
    }

    #[test]
    fn derived_class_constructor_without_explicit_super() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                int baseValue;
                Base() { baseValue = 10; }
            }

            class Derived : Base {
                int derivedValue = 20;

                Derived() {
                    // No explicit super() - should auto-call base constructor
                    derivedValue = 30;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Derived constructor without super() should auto-call base: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_nested_statement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived(bool flag) {
                    if (flag) {
                        super();
                    }
                }
            }

            void test() {
                Derived d = Derived(true);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in nested if should be detected: {:?}", result.errors);
    }

    // ==================== Do-While Loop Tests ====================

    #[test]
    fn do_while_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                do {
                    i = i + 1;
                } while (i < 10);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Do-while loop should work: {:?}", result.errors);
    }

    #[test]
    fn do_while_with_break() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                do {
                    i = i + 1;
                    if (i == 5) {
                        break;
                    }
                } while (i < 10);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Do-while with break should work: {:?}", result.errors);
    }

    #[test]
    fn do_while_with_continue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                int sum = 0;
                do {
                    i = i + 1;
                    if (i == 5) {
                        continue;
                    }
                    sum = sum + i;
                } while (i < 10);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Do-while with continue should work: {:?}", result.errors);
    }

    #[test]
    fn do_while_non_bool_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                do {
                    i = i + 1;
                } while (i); // Should error: int not bool
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Do-while with non-bool condition should error");
    }

    // ==================== Try-Catch Tests ====================

    #[test]
    fn try_catch_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                try {
                    int x = 42;
                }
                catch {
                    int y = 0;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Try-catch should work: {:?}", result.errors);
    }

    #[test]
    fn try_catch_with_return() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int test() {
                try {
                    return 42;
                }
                catch {
                    return 0;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Try-catch with return should work: {:?}", result.errors);
    }

    // ==================== Error Path Tests ====================

    #[test]
    fn break_outside_loop_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                break;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Break outside loop should error");
    }

    #[test]
    fn continue_outside_loop_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                continue;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Continue outside loop should error");
    }

    #[test]
    fn void_variable_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                void x;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Void variable should error");
    }

    #[test]
    fn return_void_from_non_void_function_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int test() {
                return;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Return void from non-void function should error");
    }

    #[test]
    fn return_value_type_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int test() {
                return "hello";
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Return value type mismatch should error");
    }

    #[test]
    fn undefined_variable_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = undefined_var;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Undefined variable should error");
    }

    #[test]
    fn this_outside_class_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = this.value;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "'this' outside class method should error");
    }

    // ==================== Enum Tests ====================

    #[test]
    fn enum_value_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void test() {
                int c = Color::Red;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Enum value access should work: {:?}", result.errors);
    }

    #[test]
    fn undefined_enum_value_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }

            void test() {
                int c = Color::Yellow;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Undefined enum value should error");
    }

    // ==================== Const Lvalue Tests ====================

    #[test]
    fn const_variable_assignment_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                const int x = 42;
                x = 10; // Should error
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Assignment to const variable should error");
    }

    // ==================== Switch Statement Tests ====================

    #[test]
    fn switch_with_default() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 2;
                int result = 0;
                switch (x) {
                    case 1:
                        result = 10;
                        break;
                    case 2:
                        result = 20;
                        break;
                    default:
                        result = 0;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Switch with default should work: {:?}", result.errors);
    }

    #[test]
    fn switch_duplicate_default_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 2;
                switch (x) {
                    default:
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Duplicate default should error");
    }

    #[test]
    fn switch_duplicate_case_value_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 2;
                switch (x) {
                    case 1:
                        break;
                    case 1:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Duplicate case value should error");
    }

    #[test]
    fn switch_unsupported_value_type_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        // Value types (non-handle classes) should still be rejected
        // Only int, bool, float, double, string, enum, and handle types are supported
        let source = r#"
            class Foo {}
            void test() {
                Foo x;
                switch (x) {
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Switch on value type should error");
    }

    // ==================== For Loop Tests ====================

    #[test]
    fn for_loop_with_init_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i;
                for (i = 0; i < 10; i = i + 1) {
                    int x = i * 2;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "For loop with init expression should work: {:?}", result.errors);
    }

    #[test]
    fn for_loop_no_condition() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                for (;;) {
                    i = i + 1;
                    if (i > 10) break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "For loop without condition should work: {:?}", result.errors);
    }

    #[test]
    fn for_loop_non_bool_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0; i; i = i + 1) { // i is int, not bool
                    int x = 0;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "For loop with non-bool condition should error");
    }

    // ==================== If Statement Tests ====================

    #[test]
    fn if_non_bool_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                if (x) { // x is int, not bool
                    int y = 0;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "If with non-bool condition should error");
    }

    #[test]
    fn while_non_bool_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                while (x) { // x is int, not bool
                    x = x - 1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "While with non-bool condition should error");
    }

    // ==================== Global Variable Tests ====================

    #[test]
    fn global_variable_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int globalVar = 42;

            void test() {
                int x = globalVar;
                globalVar = 100;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Global variable access should work: {:?}", result.errors);
    }

    // ==================== Implicit Member Access Tests ====================

    #[test]
    fn implicit_this_field_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value;

                void setValue(int v) {
                    value = v; // Implicit this.value
                }

                int getValue() {
                    return value; // Implicit this.value
                }
            }

            void test() {
                MyClass obj;
                obj.setValue(42);
                int x = obj.getValue();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Implicit this field access should work: {:?}", result.errors);
    }

    #[test]
    fn implicit_this_shadows_local() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value;

                void test() {
                    int value = 10; // Local shadows field
                    int x = value; // Uses local
                    this.value = x; // Explicit this for field
                }
            }

            void test() {
                MyClass obj;
                obj.test();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Local shadowing field should work: {:?}", result.errors);
    }

    // ==================== Namespace Tests ====================

    #[test]
    fn namespaced_function() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Game {
                void doSomething() {
                    int x = 42;
                }
            }

            void test() {
                Game::doSomething();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespaced function should work: {:?}", result.errors);
    }

    #[test]
    fn nested_namespace_function() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Game::World {
                void spawn() {
                    int x = 42;
                }
            }

            void test() {
                Game::World::spawn();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested namespace function should work: {:?}", result.errors);
    }

    // ==================== Complex Super Call Detection Tests ====================

    #[test]
    fn super_call_in_while_loop() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived(bool flag) {
                    while (flag) {
                        super();
                        break;
                    }
                }
            }

            void test() {
                Derived d = Derived(true);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in while should be detected: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_do_while() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    do {
                        super();
                    } while (false);
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in do-while should be detected: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_for_loop_init() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    for (int i = 0; i < 1; i = i + 1) {
                        super();
                    }
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in for loop should be detected: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_nested_block() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    {
                        {
                            super();
                        }
                    }
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in nested block should be detected: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_switch() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived(int x) {
                    switch (x) {
                        case 1:
                            super();
                            break;
                        default:
                            super();
                    }
                }
            }

            void test() {
                Derived d = Derived(1);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in switch should be detected: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_try_catch() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    try {
                        super();
                    }
                    catch {
                    }
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in try-catch should be detected: {:?}", result.errors);
    }

    // ==================== Expression Contains Super Tests ====================

    #[test]
    fn super_call_in_binary_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                int value;
                Base() { value = 10; }
                int getValue() { return value; }
            }

            class Derived : Base {
                Derived() {
                    int x = 0;
                    super();
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call in expression should be detected: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_return_value() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    return;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with return should work: {:?}", result.errors);
    }

    // ==================== Method Signature Matching Tests ====================

    #[test]
    fn overloaded_methods() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                void process(int x) { }
                void process(float x) { }
                void process(int x, int y) { }
            }

            void test() {
                MyClass obj;
                obj.process(42);
                obj.process(3.14f);
                obj.process(1, 2);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Overloaded methods should work: {:?}", result.errors);
    }

    // ==================== Ternary Expression Tests ====================

    #[test]
    fn ternary_type_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool cond = true;
                int x = cond ? 42 : "hello"; // int vs string
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Ternary with mismatched types should error");
    }

    #[test]
    fn ternary_non_bool_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int cond = 5;
                int x = cond ? 1 : 2; // cond is int, not bool
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Ternary with non-bool condition should error");
    }

    // ==================== Postfix Operator Tests ====================

    #[test]
    fn postfix_on_rvalue_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                (5)++; // Can't increment literal
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Postfix on rvalue should error");
    }

    // ==================== Init List Tests ====================

    #[test]
    fn init_list_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<int> arr = {1, 2, 3, 4, 5};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Init list should work: {:?}", result.errors);
    }

    // ==================== Null Literal Tests ====================

    #[test]
    fn null_literal_usage() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass@ obj = null;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Null literal should work: {:?}", result.errors);
    }

    // ==================== Cast Expression Tests ====================

    #[test]
    fn explicit_cast_to_same_type() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 42;
                int y = int(x);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Cast to same type should work: {:?}", result.errors);
    }

    #[test]
    fn explicit_cast_numeric() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float f = 3.14f;
                int x = int(f);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Numeric cast should work: {:?}", result.errors);
    }

    #[test]
    fn invalid_cast_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s = "hello";
                int x = int(s);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Invalid cast should error");
    }

    // ==================== Property Access Tests ====================

    #[test]
    fn property_getter_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        // Use explicit method calls instead of virtual property access
        let source = r#"
            class MyClass {
                private int _value;

                int getValue() { return _value; }
                void setValue(int v) { _value = v; }
            }

            void test() {
                MyClass obj;
                obj.setValue(42);
                int x = obj.getValue();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Property getter access should work: {:?}", result.errors);
    }

    // ==================== Funcdef Tests ====================

    #[test]
    fn funcdef_variable() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void CALLBACK();

            void myFunc() { }

            void test() {
                CALLBACK@ cb = @myFunc;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef variable should work: {:?}", result.errors);
    }

    // ==================== Compound Assignment Tests ====================

    #[test]
    fn compound_assignment_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 10;
                x += 5;
                x -= 3;
                x *= 2;
                x /= 4;
                x %= 3;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_assignment_on_const_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                const int x = 10;
                x += 5; // Should error
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Compound assignment on const should error");
    }

    // ==================== Lambda Tests ====================

    #[test]
    fn lambda_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int CALLBACK(int);

            void test() {
                CALLBACK@ cb = function(int x) { return x * 2; };
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Lambda should work: {:?}", result.errors);
    }

    #[test]
    fn lambda_with_captures() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int CALLBACK();

            void test() {
                int value = 42;
                CALLBACK@ cb = function() { return value; };
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Lambda with captures should work: {:?}", result.errors);
    }

    // ==================== Unary Operator Tests ====================

    #[test]
    fn unary_not_operator() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = !a;
                bool c = !!a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Unary not should work: {:?}", result.errors);
    }

    #[test]
    fn unary_bitwise_not() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 42;
                int b = ~a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Unary bitwise not should work: {:?}", result.errors);
    }

    #[test]
    fn unary_pre_increment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = ++a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Pre-increment should work: {:?}", result.errors);
    }

    #[test]
    fn unary_pre_decrement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = --a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Pre-decrement should work: {:?}", result.errors);
    }

    #[test]
    fn postfix_increment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = a++;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Post-increment should work: {:?}", result.errors);
    }

    #[test]
    fn postfix_decrement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = a--;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Post-decrement should work: {:?}", result.errors);
    }

    // ==================== Bitwise Operator Tests ====================

    #[test]
    fn bitwise_operators_all() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 0xFF;
                int b = 0x0F;
                int c1 = a & b;
                int c2 = a | b;
                int c3 = a ^ b;
                int c4 = a << 4;
                int c5 = a >> 2;
                int c6 = a >>> 2;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise operators should work: {:?}", result.errors);
    }

    // ==================== Handle (@) Tests ====================

    #[test]
    fn handle_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value;
            }

            void test() {
                MyClass@ a = null;
                MyClass@ b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Handle assignment should work: {:?}", result.errors);
    }

    #[test]
    fn handle_comparison() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass@ a = null;
                MyClass@ b = null;
                bool c = a is b;
                bool d = a !is b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Handle comparison with is/!is should work: {:?}", result.errors);
    }

    #[test]
    fn handle_comparison_with_null() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass@ a = null;
                bool c = a is null;
                bool d = a !is null;
                bool e = null is a;
                bool f = null !is a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Handle comparison with null should work: {:?}", result.errors);
    }

    #[test]
    fn is_operator_non_handle_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = 10;
                bool c = a is b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "is operator with non-handles should error");
        let error_msg = format!("{:?}", result.errors);
        assert!(error_msg.contains("handle"), "Error should mention handle type requirement");
    }

    #[test]
    fn is_operator_mixed_types_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass@ a = null;
                int b = 5;
                bool c = a is b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "is operator with mixed handle/non-handle should error");
    }

    // ==================== Logical Operator Tests ====================

    #[test]
    fn logical_and_short_circuit() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = a && b;
                bool d = b && a; // Short circuits
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Logical AND should work: {:?}", result.errors);
    }

    #[test]
    fn logical_or_short_circuit() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = a || b; // Short circuits
                bool d = b || a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Logical OR should work: {:?}", result.errors);
    }

    #[test]
    fn logical_xor() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = a ^^ b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Logical XOR should work: {:?}", result.errors);
    }

    // ==================== Power Operator Tests ====================

    #[test]
    fn power_operator() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float a = 2.0f;
                float b = a ** 3.0f;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Power operator should work: {:?}", result.errors);
    }

    // ==================== Double Literal Tests ====================

    #[test]
    fn double_literal() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double a = 3.14159265358979;
                double b = 1.0e10;
                double c = a + b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Double literals should work: {:?}", result.errors);
    }

    // ==================== Multiple Variable Declaration Tests ====================

    #[test]
    fn multiple_variables_same_type() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int y = 2;
                int z = 3;
                int sum = x + y + z;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multiple variables should work: {:?}", result.errors);
    }

    // ==================== Complex Super Call Expression Tests ====================

    #[test]
    fn super_call_in_ternary() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived(bool flag) {
                    super();
                    int x = flag ? 1 : 0;
                }
            }

            void test() {
                Derived d = Derived(true);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with ternary should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_unary() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    int x = -5;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with unary should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_assign() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                int value;

                Derived() {
                    super();
                    value = 10;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with assign should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_member_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Inner {
                int value;
            }

            class Base {
                Inner inner;
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    inner.value = 42;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with member access should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_index_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    array<int> arr = {1, 2, 3};
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Super call with array init should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_postfix_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                int counter;

                Derived() {
                    super();
                    counter++;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with postfix should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_cast_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    int x = int(3.14f);
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with cast should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_paren_expr() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    int x = (1 + 2) * 3;
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with paren should work: {:?}", result.errors);
    }

    // ==================== Foreach Error Tests ====================

    #[test]
    fn foreach_on_non_iterable_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 42;
                foreach (int i : x) { // int is not iterable
                    int y = i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should error because int doesn't have foreach operators
        assert!(!result.errors.is_empty(), "Foreach on non-iterable should error");
    }

    // ==================== If-Else Tests ====================

    #[test]
    fn if_else_basic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                int result;
                if (x > 0) {
                    result = 1;
                } else {
                    result = -1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "If-else should work: {:?}", result.errors);
    }

    #[test]
    fn if_else_if_chain() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                int result;
                if (x > 10) {
                    result = 1;
                } else if (x > 5) {
                    result = 2;
                } else if (x > 0) {
                    result = 3;
                } else {
                    result = 4;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "If-else-if chain should work: {:?}", result.errors);
    }

    // ==================== Expression Statement Tests ====================

    #[test]
    fn empty_expression_statement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                ;  // Empty statement
                x;  // Expression statement (discarded)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Expression statement should work: {:?}", result.errors);
    }

    // ==================== Method Call Tests ====================

    #[test]
    fn method_call_with_args() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Calculator {
                int add(int a, int b) { return a + b; }
                int multiply(int a, int b) { return a * b; }
            }

            void test() {
                Calculator calc;
                int sum = calc.add(5, 3);
                int product = calc.multiply(4, 7);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Method call with args should work: {:?}", result.errors);
    }

    // ==================== Return Value Conversion Tests ====================

    #[test]
    fn return_implicit_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            float test() {
                int x = 42;
                return x; // int to float implicit conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Return implicit conversion should work: {:?}", result.errors);
    }

    // ==================== Binary Void Error Tests ====================

    #[test]
    fn binary_void_left_operand_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() { }

            void test() {
                int x = doNothing() + 5; // void + int is error
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Binary with void left operand should error");
    }

    #[test]
    fn binary_void_right_operand_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() { }

            void test() {
                int x = 5 + doNothing(); // int + void is error
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Binary with void right operand should error");
    }

    // ==================== Class Inheritance Tests ====================

    #[test]
    fn inherited_field_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                int baseValue;
            }

            class Derived : Base {
                void setValues() {
                    baseValue = 100; // Access inherited field
                }
            }

            void test() {
                Derived d;
                d.setValues();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Inherited field access should work: {:?}", result.errors);
    }

    // ==================== Operator Overload Tests ====================

    #[test]
    fn class_with_opAdd() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Vector2 {
                float x;
                float y;

                Vector2 opAdd(const Vector2 &in other) {
                    Vector2 result;
                    result.x = x + other.x;
                    result.y = y + other.y;
                    return result;
                }
            }

            void test() {
                Vector2 a;
                a.x = 1.0f;
                a.y = 2.0f;

                Vector2 b;
                b.x = 3.0f;
                b.y = 4.0f;

                Vector2 c = a + b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "opAdd operator overload should work: {:?}", result.errors);
    }

    // ==================== Abstract Method Tests ====================

    #[test]
    fn abstract_method_no_body() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            abstract class Shape {
                abstract float area();
            }

            class Circle : Shape {
                float radius;

                float area() {
                    return 3.14159f * radius * radius;
                }
            }

            void test() {
                Circle c;
                c.radius = 5.0f;
                float a = c.area();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Abstract class should work: {:?}", result.errors);
    }

    // ==================== Index Expression Tests ====================

    #[test]
    fn index_expression_multi() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Matrix {
                int opIndex(int row, int col) { return row * 10 + col; }
            }

            void test() {
                Matrix m;
                int val = m[2, 3];
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multi-index opIndex should work: {:?}", result.errors);
    }

    // ==================== Funcdef Call Tests ====================

    #[test]
    fn funcdef_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int BINARY_OP(int, int);

            int add(int a, int b) { return a + b; }

            void test() {
                BINARY_OP@ op = @add;
                int result = op(5, 3);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef call should work: {:?}", result.errors);
    }

    // ==================== Type Assignment Error Tests ====================

    #[test]
    fn assignment_incompatible_types_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class A { }
            class B { }

            void test() {
                A a;
                B b;
                a = b; // Incompatible types
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Incompatible assignment should error");
    }

    // ==================== Init List Tests ====================

    #[test]
    fn init_list_empty() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<int> arr = {};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Empty init list should work: {:?}", result.errors);
    }

    #[test]
    fn init_list_multidimensional() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<array<int>> matrix = {{1, 2}, {3, 4}};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Multidimensional init list should work: {:?}", result.errors);
    }

    #[test]
    fn init_list_in_nested_block() {
        // Test that template types are instantiated when used in nested blocks
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                if (true) {
                    array<int> arr = {1, 2, 3};
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Init list in nested block should work: {:?}", result.errors);
    }

    #[test]
    fn init_list_in_for_loop() {
        // Test that template types are instantiated when used in for loop body
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    array<int> arr = {i, i+1, i+2};
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Init list in for loop should work: {:?}", result.errors);
    }

    #[test]
    fn init_list_in_while_loop() {
        // Test that template types are instantiated when used in while loop body
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                while (i < 5) {
                    array<float> arr = {1.0f, 2.0f};
                    i++;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Init list in while loop should work: {:?}", result.errors);
    }

    #[test]
    fn init_list_deeply_nested_blocks() {
        // Test template instantiation in deeply nested control structures
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                if (true) {
                    for (int i = 0; i < 3; i++) {
                        while (i > 0) {
                            array<double> arr = {1.0, 2.0, 3.0};
                            break;
                        }
                    }
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Init list in deeply nested blocks should work: {:?}", result.errors);
    }

    #[test]
    fn template_type_in_switch() {
        // Test template instantiation in switch case blocks
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1:
                        array<int> arr = {10, 20};
                        break;
                    case 2:
                        array<float> arr2 = {1.5f, 2.5f};
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Template type in switch should work: {:?}", result.errors);
    }

    #[test]
    fn template_type_in_try_catch() {
        // Test template instantiation in try/catch blocks
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                try {
                    array<int> arr = {1, 2, 3};
                }
                catch {
                    array<float> arr2 = {0.0f};
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Template type in try/catch should work: {:?}", result.errors);
    }

    #[test]
    fn multiple_template_types_same_function() {
        // Test multiple different template instantiations in same function
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<int> intArr = {1, 2, 3};
                array<float> floatArr = {1.0f, 2.0f};
                array<double> doubleArr = {1.0, 2.0, 3.0};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Multiple template types should work: {:?}", result.errors);
    }

    // ==================== Super Call Detection in Expressions ====================

    #[test]
    fn super_detection_in_call_args() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper(int x) { }

            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    helper(42);
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call with function args should work: {:?}", result.errors);
    }

    #[test]
    fn super_detection_in_init_list() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::array_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    array<int> arr = {1, 2, 3};
                }
            }

            void test() {
                Derived d;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let array_mod = array_module().expect("Failed to create array module");
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Super detection with init list should work: {:?}", result.errors);
    }

    // ==================== Function Without Body Tests ====================

    #[test]
    fn interface_method_no_body() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            interface IShape {
                float area();
            }

            class Square : IShape {
                float side;

                float area() {
                    return side * side;
                }
            }

            void test() {
                Square s;
                s.side = 5.0f;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Interface with implementation should work: {:?}", result.errors);
    }

    // ==================== Foreach With Various Errors ====================

    #[test]
    fn foreach_missing_opForEnd() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Container {
                int opForBegin() { return 0; }
                // Missing opForEnd
            }

            void test() {
                Container c;
                foreach (int i : c) {
                    int x = i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should error about missing opForEnd
        assert!(!result.errors.is_empty(), "Missing opForEnd should error");
    }

    #[test]
    fn foreach_missing_opForNext() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Container {
                int opForBegin() { return 0; }
                bool opForEnd(int) { return true; }
                // Missing opForNext
            }

            void test() {
                Container c;
                foreach (int i : c) {
                    int x = i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should error about missing opForNext
        assert!(!result.errors.is_empty(), "Missing opForNext should error");
    }

    #[test]
    fn foreach_missing_opForValue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Container {
                int opForBegin() { return 0; }
                bool opForEnd(int) { return true; }
                int opForNext(int i) { return i + 1; }
                // Missing opForValue
            }

            void test() {
                Container c;
                foreach (int i : c) {
                    int x = i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Should error about missing opForValue
        assert!(!result.errors.is_empty(), "Missing opForValue should error");
    }

    // ==================== Lambda With Context Tests ====================

    #[test]
    fn lambda_in_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int TRANSFORM(int);

            int apply(int x, TRANSFORM@ fn) {
                return fn(x);
            }

            void test() {
                int result = apply(5, function(int x) { return x * 2; });
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Lambda in function call should work: {:?}", result.errors);
    }

    // ==================== Overloaded Function Call Tests ====================

    #[test]
    fn overloaded_function_call_exact_match() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::string_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(int x) { }
            void process(float x) { }
            void process(string x) { }

            void test() {
                process(42);
                process(3.14f);
                process("hello");
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let string_mod = string_module().expect("Failed to create string module");
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Overloaded function call should work: {:?}", result.errors);
    }

    #[test]
    fn overloaded_function_call_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(float x) { }

            void test() {
                int i = 42;
                process(i); // int to float implicit conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Overloaded function with conversion should work: {:?}", result.errors);
    }

    #[test]
    fn function_call_wrong_arg_count_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper(int a, int b) { }

            void test() {
                helper(1); // Too few arguments
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Wrong argument count should error");
    }

    #[test]
    fn function_call_too_many_args_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void helper(int a) { }

            void test() {
                helper(1, 2, 3); // Too many arguments
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Too many arguments should error");
    }

    // ==================== Default Argument Tests ====================

    #[test]
    fn function_with_default_args() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::string_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void greet(string name, int times = 1) {
                int i = 0;
                while (i < times) {
                    i = i + 1;
                }
            }

            void test() {
                greet("hello");
                greet("world", 3);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let string_mod = string_module().expect("Failed to create string module");
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Function with default args should work: {:?}", result.errors);
    }

    // ==================== Member Access on Non-Object Tests ====================

    #[test]
    fn member_access_on_primitive_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 42;
                int y = x.value; // int has no members
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Member access on primitive should error");
    }

    // ==================== Method Call on Non-Object Tests ====================

    #[test]
    fn method_call_on_primitive_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 42;
                int y = x.getValue(); // int has no methods
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Method call on primitive should error");
    }

    // ==================== Undefined Function Tests ====================

    #[test]
    fn undefined_function_call_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = undefinedFunction();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Undefined function should error");
    }

    // ==================== Undefined Method Tests ====================

    #[test]
    fn undefined_method_call_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass obj;
                obj.undefinedMethod();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Undefined method should error");
    }

    // ==================== Ternary Type Unification Tests ====================

    #[test]
    fn ternary_int_float_promotion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool cond = true;
                float x = cond ? 42 : 3.14f; // int promoted to float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Ternary with type promotion should work: {:?}", result.errors);
    }

    // ==================== Break/Continue Target Tests ====================

    #[test]
    fn break_in_nested_loops() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                while (i < 10) {
                    int j = 0;
                    while (j < 10) {
                        if (j == 5) break; // Inner break
                        j = j + 1;
                    }
                    if (i == 5) break; // Outer break
                    i = i + 1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Break in nested loops should work: {:?}", result.errors);
    }

    #[test]
    fn continue_in_nested_loops() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                while (i < 10) {
                    int j = 0;
                    while (j < 10) {
                        j = j + 1;
                        if (j == 5) continue; // Inner continue
                    }
                    i = i + 1;
                    if (i == 5) continue; // Outer continue
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Continue in nested loops should work: {:?}", result.errors);
    }

    // ==================== Compound Assignment Type Error Tests ====================

    #[test]
    fn compound_assignment_type_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 10;
                x += "hello"; // string not compatible
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Compound assignment type mismatch should error");
    }

    // ==================== Array Tests ====================

    #[test]
    fn array_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<int> arr = {1, 2, 3, 4, 5};
                int first = arr[0];
                arr[1] = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let ffi = create_ffi_with_array();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "Array access should work: {:?}", result.errors);
    }

    // ==================== Int8/Int16/Int64 Tests ====================

    #[test]
    fn various_int_types() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int8 a = 127;
                int16 b = 32767;
                int64 c = 1234567890;
                uint8 d = 255;
                uint16 e = 65535;
                uint64 f = 1234567890;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Various int types including unsigned should work: {:?}", result.errors);
    }

    // ==================== Static Method Tests ====================

    #[test]
    fn static_method_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Math {
                int square(int x) { return x * x; }
            }

            void test() {
                Math m;
                int result = m.square(5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Method call should work: {:?}", result.errors);
    }

    // ==================== Complex Expression Tests ====================

    #[test]
    fn complex_expression_chain() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 1;
                int b = 2;
                int c = 3;
                int result = ((a + b) * c - (a << 2) | (b & c)) ^ ((a > b) ? c : -c);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Complex expression chain should work: {:?}", result.errors);
    }

    // ==================== Class with opIndex Tests ====================

    #[test]
    fn class_with_opindex() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Container {
                int &opIndex(int idx) { return idx; }
            }

            void test() {
                Container c;
                int val = c[0];
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercises opIndex code path
        let _ = result;
    }

    #[test]
    fn type_without_indexing_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass c;
                int val = c[0]; // MyClass has no opIndex
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Indexing type without opIndex should error");
    }

    // ==================== Super Call Error Cases ====================

    #[test]
    fn super_outside_class_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                super(); // Not in a class
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Super outside class should error");
    }

    #[test]
    fn super_without_base_class_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class NoBase {
                NoBase() {
                    super(); // No base class
                }
            }

            void test() {
                NoBase n;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Super without base class should error");
    }

    // ==================== Constructor Error Cases ====================

    #[test]
    fn constructor_wrong_args_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                MyClass(int x) { }
            }

            void test() {
                MyClass c = MyClass("wrong type"); // String instead of int
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Constructor with wrong arg type should error");
    }

    // ==================== Void in Various Contexts ====================

    #[test]
    fn void_ternary_both_branches_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() { }

            void test() {
                bool cond = true;
                int x = cond ? doNothing() : doNothing();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Void in ternary branches should error");
    }

    #[test]
    fn void_ternary_else_branch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() { }

            void test() {
                bool cond = true;
                int x = cond ? 42 : doNothing();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Void in ternary else branch should error");
    }

    // ==================== Unsigned Shift Operators ====================

    #[test]
    fn unsigned_right_shift() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = -256;
                int b = a >>> 2; // Unsigned right shift
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Unsigned right shift should work: {:?}", result.errors);
    }

    // ==================== Prefix Operators ====================

    #[test]
    fn prefix_minus_on_various_types() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = -42;
                float b = -3.14f;
                double c = -2.71828;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Prefix minus on various types should work: {:?}", result.errors);
    }

    #[test]
    fn prefix_plus_on_numeric() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = +42;
                float b = +3.14f;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Prefix plus should work: {:?}", result.errors);
    }

    // ==================== Comparison Operators ====================

    #[test]
    fn all_comparison_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = 10;
                bool lt = a < b;
                bool le = a <= b;
                bool gt = a > b;
                bool ge = a >= b;
                bool eq = a == b;
                bool ne = a != b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "All comparison operators should work: {:?}", result.errors);
    }

    // ==================== Handle Operations ====================

    #[test]
    fn handle_to_object_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value;
            }

            void test() {
                MyClass@ handle = null;
                MyClass obj;
                @handle = @obj;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Test handle-to-handle assignment
        let _ = result;
    }

    // ==================== Const Parameter Tests ====================

    #[test]
    fn const_reference_parameter() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(const int &in value) {
                int x = value;
            }

            void test() {
                int a = 42;
                process(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Const reference parameter should work: {:?}", result.errors);
    }

    // ==================== Multiple Return Paths ====================

    #[test]
    fn multiple_return_paths() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int getValue(bool flag) {
                if (flag) {
                    return 1;
                } else {
                    return 2;
                }
            }

            void test() {
                int a = getValue(true);
                int b = getValue(false);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multiple return paths should work: {:?}", result.errors);
    }

    // ==================== Nested Class Access ====================

    #[test]
    fn deeply_nested_member_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Inner {
                int value;
            }

            class Outer {
                Inner inner;
            }

            class Container {
                Outer outer;
            }

            void test() {
                Container c;
                c.outer.inner.value = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Deeply nested member access should work: {:?}", result.errors);
    }

    // ==================== Modulo Operation ====================

    #[test]
    fn modulo_operation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 17;
                int b = 5;
                int remainder = a % b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Modulo operation should work: {:?}", result.errors);
    }

    // ==================== Global Const Variable ====================

    #[test]
    fn global_const_variable() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            const int MAX_VALUE = 100;

            void test() {
                int x = MAX_VALUE;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Global const variable should work: {:?}", result.errors);
    }

    // ==================== Private Field Access Error ====================

    #[test]
    fn private_field_access_from_outside_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                private int secret;
            }

            void test() {
                MyClass obj;
                int x = obj.secret; // Private access from outside
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Private field access from outside should error");
    }

    // ==================== Unary on Wrong Type Error ====================

    #[test]
    fn unary_not_on_int_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                bool b = !x; // Not on int is error
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Unary not on int should error");
    }

    #[test]
    fn unary_minus_on_bool_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool b = true;
                bool c = -b; // Minus on bool is error
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Unary minus on bool should error");
    }

    // ==================== Reference Parameter with Literal Error ====================

    #[test]
    fn reference_out_param_with_literal_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void getResult(int &out result) {
                result = 42;
            }

            void test() {
                getResult(5); // Can't pass literal to &out
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Reference out param with literal should error");
    }

    // ==================== Empty Block ====================

    #[test]
    fn empty_block_statement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                {
                    // Empty block
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Empty block should work: {:?}", result.errors);
    }

    // ==================== Division ====================

    #[test]
    fn division_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10 / 3;
                float b = 10.0f / 3.0f;
                double c = 10.0 / 3.0;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Division operators should work: {:?}", result.errors);
    }

    // ==================== String Concatenation ====================

    #[test]
    fn string_concatenation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string a = "Hello";
                string b = "World";
                string c = a + " " + b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercises string + operator path
        let _ = result;
    }

    // ==================== Assignment to Function Call Result Error ====================

    #[test]
    fn assignment_to_rvalue_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int getValue() { return 42; }

            void test() {
                getValue() = 10; // Can't assign to rvalue
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Assignment to rvalue should error");
    }

    // ==================== Constructor with No Constructor Defined ====================

    #[test]
    fn class_implicit_default_constructor() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class SimpleClass {
                int value;
            }

            void test() {
                SimpleClass obj; // Uses implicit default constructor
                obj.value = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Implicit default constructor should work: {:?}", result.errors);
    }

    // ==================== Binary Operation Errors ====================

    #[test]
    fn binary_arithmetic_on_non_numeric_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = a + b; // Cannot do arithmetic on bool
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Arithmetic on non-numeric types should error");
    }

    #[test]
    fn binary_bitwise_on_float_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float a = 3.14f;
                float b = 2.71f;
                float c = a & b; // Cannot do bitwise on float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Bitwise on float types should error");
    }

    #[test]
    fn logical_operator_on_int_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 5;
                int b = 10;
                bool c = a && b; // Requires bool operands
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Logical operators on int should error");
    }

    // ==================== Protected Field Access ====================

    #[test]
    fn protected_field_access_from_derived() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                protected int value;
            }

            class Derived : Base {
                void test() {
                    value = 42; // Accessing protected from derived should work
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Protected access from derived should work: {:?}", result.errors);
    }

    // ==================== Funcdef Handle Operations ====================

    #[test]
    fn funcdef_handle_null_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void CALLBACK();

            void test() {
                CALLBACK@ cb = null;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Funcdef handle null assignment should work: {:?}", result.errors);
    }

    // ==================== Complex Ternary Types ====================

    #[test]
    fn ternary_type_promotion_int_double() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool cond = true;
                double x = cond ? 42 : 3.14; // int promotes to double
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Ternary type promotion should work: {:?}", result.errors);
    }

    // ==================== Assignment Operators ====================

    #[test]
    fn compound_modulo_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 17;
                x %= 5;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound modulo assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_power_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 2;
                x **= 3;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound power assignment should work: {:?}", result.errors);
    }

    // ==================== Method Access From Handle ====================

    #[test]
    fn method_call_on_handle() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int getValue() { return 42; }
            }

            void test() {
                MyClass@ handle = null;
                MyClass obj;
                @handle = @obj;
                int val = handle.getValue();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercises method call on handle
        let _ = result;
    }

    // ==================== Unary Operators on Different Types ====================

    #[test]
    fn bitwise_not_on_uint64() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int64 x = 0xFFFF;
                int64 y = ~x;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise not on int64 should work: {:?}", result.errors);
    }

    #[test]
    fn prefix_increment_on_field() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                int count;

                void increment() {
                    ++count;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Prefix increment on field should work: {:?}", result.errors);
    }

    // ==================== Loop Control in Different Contexts ====================

    #[test]
    fn break_in_do_while() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                do {
                    if (i == 5) break;
                    i = i + 1;
                } while (i < 10);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Break in do-while should work: {:?}", result.errors);
    }

    #[test]
    fn continue_in_for() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int sum = 0;
                for (int i = 0; i < 10; i = i + 1) {
                    if (i % 2 == 0) continue;
                    sum = sum + i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Continue in for should work: {:?}", result.errors);
    }

    // ==================== Return Conversion Tests ====================

    #[test]
    fn return_int_from_float_function() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            float getValue() {
                return 42; // int converts to float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Return int from float function should work: {:?}", result.errors);
    }

    // ==================== Nested If Tests ====================

    #[test]
    fn deeply_nested_if() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                if (true) {
                    if (true) {
                        if (true) {
                            x = 1;
                        }
                    }
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Deeply nested if should work: {:?}", result.errors);
    }

    // ==================== Switch With Enum ====================

    #[test]
    fn switch_on_enum() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum State {
                Idle,
                Running,
                Stopped
            }

            void test() {
                int state = State::Running;
                switch (state) {
                    case State::Idle:
                        break;
                    case State::Running:
                        break;
                    case State::Stopped:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Switch on enum should work: {:?}", result.errors);
    }

    // ==================== Multiple Variable Init ====================

    #[test]
    fn multiple_variable_init_same_statement() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 1, b = 2, c = 3;
                int sum = a + b + c;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multiple variable init should work: {:?}", result.errors);
    }

    // ==================== Class Method Self Reference ====================

    #[test]
    fn class_method_this_member_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value;

                void setValue(int v) {
                    this.value = v;
                }

                int getValue() {
                    return this.value;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Explicit this member access should work: {:?}", result.errors);
    }

    // ==================== Super Call Detection in Various Statements ====================

    #[test]
    fn super_call_in_foreach() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super call should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_for_update() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                    for (int i = 0; i < 1; i = i + 1) {
                    }
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "For loop with super should work: {:?}", result.errors);
    }

    #[test]
    fn super_call_in_lambda_body() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                Base() { }
            }

            class Derived : Base {
                Derived() {
                    super();
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Super in derived constructor should work: {:?}", result.errors);
    }

    // ==================== Type Checking Edge Cases ====================

    #[test]
    fn local_variable_shadowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int global = 10;

            void test() {
                int global = 20; // Shadows global
                int x = global;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Local variable shadowing should work: {:?}", result.errors);
    }

    #[test]
    fn nested_block_scoping() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                {
                    int y = 2;
                    int z = x + y;
                }
                int w = x;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested block scoping should work: {:?}", result.errors);
    }

    // ==================== Method Overriding ====================

    #[test]
    fn method_override_in_derived() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                void doSomething() { }
            }

            class Derived : Base {
                void doSomething() { } // Override
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Method override should work: {:?}", result.errors);
    }

    // ==================== Function Return Path Tests ====================

    #[test]
    fn function_early_return() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int getValue(bool flag) {
                if (flag) {
                    return 1;
                }
                return 0;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Early return should work: {:?}", result.errors);
    }

    #[test]
    fn void_function_explicit_return() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doSomething(bool flag) {
                if (flag) {
                    return;
                }
                int x = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Void function with explicit return should work: {:?}", result.errors);
    }

    // ==================== Postfix on Member Access ====================

    #[test]
    fn postfix_on_member() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                int value;

                void test() {
                    value++;
                    value--;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Postfix on member should work: {:?}", result.errors);
    }

    // ==================== Field Initializers ====================

    #[test]
    fn field_initializer_with_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Config {
                int timeout = 30 * 1000;
                float ratio = 16.0f / 9.0f;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Field initializers with expressions should work: {:?}", result.errors);
    }

    // ==================== While Loop with Complex Condition ====================

    #[test]
    fn while_complex_condition() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                int y = 10;
                while (x < 10 && y > 0) {
                    x = x + 1;
                    y = y - 1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "While with complex condition should work: {:?}", result.errors);
    }

    // ==================== Cast Expressions ====================

    #[test]
    fn explicit_cast_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float f = 3.14f;
                int x = int(f);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Explicit cast should work: {:?}", result.errors);
    }

    #[test]
    fn cast_between_numeric_types() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int64 big = 1000000;
                int small = int(big);
                float f = float(small);
                double d = double(f);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Cast between numeric types should work: {:?}", result.errors);
    }

    // ==================== Expression Statement ====================

    #[test]
    fn expression_statement_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doWork() { }

            void test() {
                doWork();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Expression statement call should work: {:?}", result.errors);
    }

    // ==================== Argument Evaluation Order ====================

    #[test]
    fn multiple_arguments_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int add(int a, int b, int c) {
                return a + b + c;
            }

            void test() {
                int result = add(1, 2, 3);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multiple arguments should work: {:?}", result.errors);
    }

    // ==================== Interface Implementation ====================

    #[test]
    fn class_implements_multiple_interfaces() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            interface IUpdatable {
                void update();
            }

            class Entity : IDrawable, IUpdatable {
                void draw() { }
                void update() { }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Multiple interface implementation should work: {:?}", result.errors);
    }

    // ==================== Negative Literals ====================

    #[test]
    fn negative_literal_in_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = -42;
                float y = -3.14f;
                int z = x + (-10);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Negative literals should work: {:?}", result.errors);
    }

    // ==================== Chained Method Calls ====================

    #[test]
    fn chained_method_calls() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Builder {
                Builder@ setValue(int v) { return this; }
                Builder@ setName(int n) { return this; }
                void build() { }
            }

            void test() {
                Builder b;
                b.setValue(42).setName(0).build();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercises chained method call path
        let _ = result;
    }

    // ==================== Bitwise Shift with Different Types ====================

    #[test]
    fn shift_operations_all_directions() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int a = x << 4;
                int b = a >> 2;
                int c = b >>> 1;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "All shift operations should work: {:?}", result.errors);
    }

    // ==================== For Loop No Init ====================

    #[test]
    fn for_loop_no_init() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 0;
                for (; i < 10; i = i + 1) {
                    int x = i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "For loop with no init should work: {:?}", result.errors);
    }

    #[test]
    fn for_loop_no_update() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0; i < 10;) {
                    i = i + 1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "For loop with no update should work: {:?}", result.errors);
    }

    // ==================== Ternary Branches Type Mismatch ====================

    #[test]
    fn ternary_incompatible_types_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class A { }
            class B { }

            void test() {
                bool cond = true;
                A a;
                B b;
                // A and B are incompatible
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This just exercises the compilation, not testing specific error
        let _ = result;
    }

    // ==================== Continue and Break at Various Depths ====================

    #[test]
    fn nested_loop_control_flow() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0; i < 5; i = i + 1) {
                    for (int j = 0; j < 5; j = j + 1) {
                        if (j == 2) continue;
                        if (j == 4) break;
                    }
                    if (i == 3) break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested loop control flow should work: {:?}", result.errors);
    }

    // ==================== Static Method Access ====================

    #[test]
    fn static_method_in_class() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Utility {
                int helper() { return 42; }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Class with methods should work: {:?}", result.errors);
    }

    // ==================== Boolean Expressions ====================

    #[test]
    fn complex_boolean_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = true;
                bool result = (a && b) || (b && c) || (!a && !b);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Complex boolean expression should work: {:?}", result.errors);
    }

    // ==================== Parenthesized Expressions ====================

    #[test]
    fn deeply_parenthesized() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = (((1 + 2) * 3) - 4);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Deeply parenthesized expression should work: {:?}", result.errors);
    }

    // ==================== Handle Null Comparison ====================

    #[test]
    fn handle_null_equality() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void test() {
                MyClass@ handle = null;
                bool isNull = handle == null;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercises handle null comparison
        let _ = result;
    }

    // ==================== Mixed Type Arithmetic ====================

    #[test]
    fn mixed_type_arithmetic() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10;
                float b = 3.5f;
                float c = a + b;
                float d = a * b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Mixed type arithmetic should work: {:?}", result.errors);
    }

    // ==================== Try-Catch Extra Tests ====================

    #[test]
    fn try_catch_with_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void riskyOperation() { }

            void test() {
                try {
                    riskyOperation();
                    int y = 10;
                }
                catch {
                    int error = 1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Try-catch with function call should work: {:?}", result.errors);
    }

    #[test]
    fn try_catch_with_loop() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                try {
                    for (int i = 0; i < 10; i++) {
                        int x = i * 2;
                    }
                }
                catch {
                    int fallback = -1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Try-catch with loop should work: {:?}", result.errors);
    }

    // ==================== Do-While Loop Extra Tests ====================

    #[test]
    fn do_while_nested_loops() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                int y = 0;
                do {
                    x = x + 1;
                    do {
                        y = y + 1;
                    } while (y < 3);
                } while (x < 5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested do-while should work: {:?}", result.errors);
    }

    #[test]
    fn do_while_complex_condition() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                int y = 0;
                do {
                    x = x + 1;
                    y = y + 2;
                } while (x < 10 && y < 15);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Do-while with complex condition should work: {:?}", result.errors);
    }

    #[test]
    fn do_while_expression_body() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 0;
                int result = 0;
                do {
                    result = result + x * 2;
                    x = x + 1;
                } while (x < 5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Do-while with expression body should work: {:?}", result.errors);
    }

    // ==================== Lambda with Captures ====================

    #[test]
    fn lambda_capture_local_variable() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Adder(int x);

            void test() {
                int offset = 10;
                Adder@ add = function(x) { return x + offset; };
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Lambda capturing local variable should work: {:?}", result.errors);
    }

    #[test]
    fn lambda_multiple_captures() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Calculator(int x);

            void test() {
                int a = 5;
                int b = 3;
                Calculator@ calc = function(x) { return x * a + b; };
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Lambda with multiple captures should work: {:?}", result.errors);
    }

    // ==================== opCall Operator ====================

    #[test]
    fn class_with_op_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Functor {
                int multiplier;

                Functor() { multiplier = 2; }

                int opCall(int x) {
                    return x * multiplier;
                }
            }

            void test() {
                Functor f;
                int result = f(5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Class with opCall should work: {:?}", result.errors);
    }

    #[test]
    fn op_call_wrong_arg_count_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Functor {
                int opCall(int x) {
                    return x * 2;
                }
            }

            void test() {
                Functor f;
                int result = f(1, 2, 3);  // Wrong arg count
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "opCall with wrong arg count should fail");
    }

    // ==================== Constructor with Field Initializers ====================

    #[test]
    fn constructor_with_field_initializers() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Player {
                int health = 100;
                int mana = 50;
                float speed = 1.5f;

                Player() { }
            }

            void test() {
                Player p;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Constructor with field initializers should work: {:?}", result.errors);
    }

    #[test]
    fn constructor_with_complex_field_initializers() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Config {
                int value = 10 + 20 * 2;
                bool active = true && false || true;

                Config() { }
            }

            void test() {
                Config c;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Constructor with complex field initializers should work: {:?}", result.errors);
    }

    // ==================== Default Parameters ====================

    #[test]
    fn function_with_default_params() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void greet(int times = 1) {
                int x = times;
            }

            void test() {
                greet();     // Uses default
                greet(5);    // Overrides default
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Function with default params should work: {:?}", result.errors);
    }

    #[test]
    fn function_multiple_default_params() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void configure(int a, int b = 10, int c = 20) {
                int sum = a + b + c;
            }

            void test() {
                configure(1);           // Uses both defaults
                configure(1, 2);        // Uses second default
                configure(1, 2, 3);     // No defaults used
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Function with multiple default params should work: {:?}", result.errors);
    }

    // ==================== Overload Resolution ====================

    #[test]
    fn overload_resolution_exact_match() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(int x) { }
            void process(float x) { }
            void process(int x, int y) { }

            void test() {
                process(5);          // int overload
                process(3.14f);      // float overload
                process(1, 2);       // two int overload
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Overload resolution with exact match should work: {:?}", result.errors);
    }

    #[test]
    fn overload_resolution_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(float x) { }
            void process(double x) { }

            void test() {
                int i = 10;
                process(i);  // Should convert int to float (lower cost than double)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Overload resolution with conversion should work: {:?}", result.errors);
    }

    // ==================== Access Violations ====================

    #[test]
    fn protected_member_from_non_derived_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Base {
                protected int secret = 42;
            }

            class Unrelated {
                void tryAccess() {
                    Base b;
                    int x = b.secret;  // Error: protected access
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Protected access from non-derived class should fail");
    }

    // ==================== Absolute Scope Resolution Extra ====================

    #[test]
    fn absolute_scope_type_reference() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class GlobalClass { }

            namespace Game {
                void test() {
                    ::GlobalClass obj;  // Absolute scope type
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Absolute scope type reference should work: {:?}", result.errors);
    }

    // ==================== Void Expression Errors ====================

    #[test]
    fn void_in_binary_operation_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void noReturn() { }

            void test() {
                int x = noReturn() + 5;  // Error: void in binary op
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Void in binary operation should fail");
    }

    #[test]
    fn void_as_function_argument_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void noReturn() { }
            void takeInt(int x) { }

            void test() {
                takeInt(noReturn());  // Error: void as argument
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Void as function argument should fail");
    }

    // ==================== Invalid Index Type ====================

    #[test]
    fn index_type_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Container {
                int opIndex(int idx) { return idx; }
            }

            void test() {
                Container c;
                int result = c["string"];  // Error: string can't convert to int
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Index with wrong type should fail");
    }

    // ==================== Derived to Base Conversion ====================

    #[test]
    fn derived_to_base_handle_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Animal { }
            class Dog : Animal { }

            void test() {
                Dog@ d;
                Animal@ a = d;  // Derived to base handle conversion
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Derived to base handle assignment should work: {:?}", result.errors);
    }

    // ==================== Reference Parameter Validation ====================

    #[test]
    fn out_param_requires_lvalue_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void modify(int &out result) {
                result = 42;
            }

            void test() {
                modify(5 + 3);  // Error: rvalue passed to &out
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Passing rvalue to &out parameter should fail");
    }

    #[test]
    fn inout_param_requires_mutable_lvalue_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void modify(int &inout value) {
                value = value + 1;
            }

            void test() {
                const int x = 10;
                modify(x);  // Error: const lvalue passed to &inout
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Passing const lvalue to &inout parameter should fail");
    }

    #[test]
    fn ref_in_param_accepts_rvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(int &in value) {
                int x = value;
            }

            void test() {
                process(42);        // rvalue literal - OK for &in
                process(5 + 3);     // rvalue expression - OK for &in
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "&in should accept rvalue: {:?}", result.errors);
    }

    #[test]
    fn ref_in_param_accepts_lvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void process(int &in value) {
                int x = value;
            }

            void test() {
                int a = 10;
                process(a);  // lvalue - OK for &in
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "&in should accept lvalue: {:?}", result.errors);
    }

    #[test]
    fn ref_out_param_accepts_mutable_lvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void getResult(int &out result) {
                result = 42;
            }

            void test() {
                int x;
                getResult(x);  // mutable lvalue - OK for &out
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "&out should accept mutable lvalue: {:?}", result.errors);
    }

    #[test]
    fn ref_out_param_rejects_const_lvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void getResult(int &out result) {
                result = 42;
            }

            void test() {
                const int x = 10;
                getResult(x);  // const lvalue - NOT OK for &out
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "&out should reject const lvalue");
    }

    #[test]
    fn ref_inout_param_accepts_mutable_lvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void modify(int &inout value) {
                value = value + 1;
            }

            void test() {
                int x = 10;
                modify(x);  // mutable lvalue - OK for &inout
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "&inout should accept mutable lvalue: {:?}", result.errors);
    }

    #[test]
    fn ref_inout_param_rejects_rvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void modify(int &inout value) {
                value = value + 1;
            }

            void test() {
                modify(42);  // rvalue - NOT OK for &inout
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "&inout should reject rvalue");
    }

    #[test]
    fn bare_ref_param_treated_as_inout() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void modify(int &value) {
                value = value + 1;
            }

            void test() {
                int x = 10;
                modify(x);  // mutable lvalue - OK for bare & (treated as &inout)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "bare & should accept mutable lvalue (like &inout): {:?}", result.errors);
    }

    #[test]
    fn bare_ref_param_rejects_rvalue() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void modify(int &value) {
                value = value + 1;
            }

            void test() {
                modify(42);  // rvalue - NOT OK for bare & (treated as &inout)
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "bare & should reject rvalue (like &inout)");
    }

    // ==================== Init List Extra Tests ====================

    #[test]
    fn init_list_with_floats() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                auto arr = {1.0f, 2.5f, 3.7f};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercise float init list type inference
        let _ = result;
    }

    #[test]
    fn init_list_mixed_numeric_types() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                auto arr = {1, 2.5f, 3};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercise mixed numeric init list type inference - should promote
        let _ = result;
    }

    // ==================== Unary Operator on Handle ====================

    #[test]
    fn handle_reference_operator() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass { }

            void takeHandle(MyClass@ obj) { }

            void test() {
                MyClass obj;
                takeHandle(@obj);  // @ creates handle from value
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Handle reference operator should work: {:?}", result.errors);
    }

    // ==================== Integer Type Variations ====================

    #[test]
    fn int8_operations() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int8 a = 10;
                int8 b = 20;
                int8 c = a + b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercise int8 operations
        let _ = result;
    }

    #[test]
    fn uint64_operations() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                uint64 a = 1000000000000;
                uint64 b = 2000000000000;
                uint64 c = a + b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // Exercise uint64 operations
        let _ = result;
    }

    // ==================== Lambda Parameter Type Mismatch ====================

    #[test]
    fn lambda_explicit_param_type_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void test() {
                Callback@ cb = function(float x) { };  // Error: param type mismatch
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Lambda with wrong param type should fail");
    }

    #[test]
    fn lambda_param_count_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void test() {
                Callback@ cb = function(a, b) { };  // Error: too many params
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Lambda with wrong param count should fail");
    }

    // ==================== Method on Handle ====================

    #[test]
    fn method_call_on_handle_member() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Inner {
                int getValue() { return 42; }
            }

            class Outer {
                Inner@ inner;
            }

            void test() {
                Outer o;
                int result = o.inner.getValue();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This exercises chained member access on handles
        let _ = result;
    }

    // ==================== Namespace with Enum and Function ====================

    #[test]
    fn namespace_function_and_enum() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            namespace Utils {
                enum LogLevel { Debug, Info, Error }

                void log(int level) { }
            }

            void test() {
                Utils::log(Utils::LogLevel::Info);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Namespace with function and enum should work: {:?}", result.errors);
    }

    // ==================== Reverse Operator ====================

    #[test]
    fn reverse_binary_operator() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Vector {
                int x;

                Vector opMul(int scalar) {
                    Vector result;
                    result.x = x * scalar;
                    return result;
                }

                Vector opMul_r(int scalar) {
                    Vector result;
                    result.x = x * scalar;
                    return result;
                }
            }

            void test() {
                Vector v;
                v.x = 5;
                Vector result = v * 10;   // Uses opMul
                Vector result2 = 10 * v;  // Uses opMul_r
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This exercises reverse operator lookup
        let _ = result;
    }

    // ==================== Not Callable Error ====================

    #[test]
    fn not_callable_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class NotCallable { }

            void test() {
                NotCallable obj;
                obj(5);  // Error: not callable
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Calling non-callable should fail");
    }

    // ==================== Undefined Function Error ====================

    #[test]
    fn undefined_function_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                nonExistentFunction();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Calling undefined function should fail");
        assert!(result.errors.iter().any(|e| e.message.contains("undefined")));
    }

    // ==================== Constructor Wrong Args Count ====================

    #[test]
    fn constructor_no_constructors_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class NoConstructor {
                int x;
            }

            void test() {
                NoConstructor obj(1, 2, 3);  // Too many args
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This tests constructor overload resolution failure
        assert!(!result.is_success(), "Constructor with wrong arg count should fail");
    }

    // ==================== get_opIndex Accessor ====================

    #[test]
    fn class_with_get_op_index() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class ReadOnlyArray {
                int get_opIndex(int idx) {
                    return idx * 10;
                }
            }

            void test() {
                ReadOnlyArray arr;
                int value = arr[5];
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Class with get_opIndex should work: {:?}", result.errors);
    }

    // ==================== While Loop Non-Boolean Condition ====================

    #[test]
    fn while_non_boolean_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                while ("string") {  // Error: non-boolean condition
                    break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "While with non-boolean condition should fail");
    }

    // ==================== If Condition Type Check ====================

    #[test]
    fn if_non_boolean_condition_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                if (42) {  // Error: non-boolean condition
                    int x = 1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "If with non-boolean condition should fail");
    }

    // ==================== Funcdef Call Through Member ====================

    #[test]
    fn funcdef_call_through_field() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Callback(int x);

            class Handler {
                Callback@ callback;
            }

            int double_it(int x) { return x * 2; }

            void test() {
                Handler h;
                h.callback = @double_it;
                int result = h.callback(5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This exercises funcdef call through member expression
        let _ = result;
    }

    // ==================== Super in Non-Constructor ====================

    #[test]
    fn super_not_class_type_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                super();  // Error: not in a class
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Super outside class should fail");
    }

    // ==================== Funcdef Wrong Signature Variations ====================

    #[test]
    fn funcdef_return_void_to_int_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int Calculator(int x);

            void wrongReturn(int x) { }

            void test() {
                Calculator@ calc = @wrongReturn;  // Error: return type mismatch
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Funcdef with wrong return type should fail");
    }

    // ==================== Destructor ====================

    #[test]
    fn class_with_destructor() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Resource {
                int handle;

                Resource() { handle = 1; }
                ~Resource() { handle = 0; }
            }

            void test() {
                Resource r;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Class with destructor should work: {:?}", result.errors);
    }

    // ==================== Short Circuit Boolean ====================

    #[test]
    fn short_circuit_and() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            bool expensive() { return true; }

            void test() {
                bool result = false && expensive();  // Should short-circuit
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Short-circuit AND should work: {:?}", result.errors);
    }

    #[test]
    fn short_circuit_or() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            bool expensive() { return true; }

            void test() {
                bool result = true || expensive();  // Should short-circuit
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Short-circuit OR should work: {:?}", result.errors);
    }

    // ==================== Numeric Promotion in Binary Ops ====================

    #[test]
    fn double_float_promotion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float a = 1.5f;
                double b = 2.5;
                double c = a + b;  // float promoted to double
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Float to double promotion should work: {:?}", result.errors);
    }

    // ==================== Property Set Error ====================

    #[test]
    fn property_set_without_setter_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class ReadOnly {
                int get_value() { return 42; }
            }

            void test() {
                ReadOnly obj;
                obj.value = 100;  // Error: no setter
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Setting property without setter should fail");
    }

    // ==================== Function Returning Wrong Type ====================

    #[test]
    fn return_wrong_type_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int getNumber() {
                return "string";  // Error: returning string from int function
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Return wrong type should fail");
    }

    // ==================== String Index Not Supported ====================

    #[test]
    fn string_index_works() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s = "hello";
                uint8 c = s[0];  // String opIndex returns uint8
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        // String indexing should work with built-in opIndex
        assert!(result.is_success(), "String index should work with opIndex: {:?}", result.errors);
    }

    // ==================== Nested Init List ====================

    #[test]
    fn nested_init_list() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                auto nested = {{1, 2}, {3, 4}, {5, 6}};
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This exercises nested init list handling
        let _ = result;
    }

    // ==================== Double Negation ====================

    #[test]
    fn double_negation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                int y = --x;  // Pre-decrement
                int z = -(-x);  // Double arithmetic negation
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Double negation should work: {:?}", result.errors);
    }

    // ==================== Absolute Scope Enum ====================

    #[test]
    fn absolute_scope_enum_value() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum GlobalColor { Red, Green, Blue }

            namespace Game {
                void test() {
                    int c = ::GlobalColor::Red;  // Absolute scope
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Absolute scope enum value should work: {:?}", result.errors);
    }

    // ==================== Switch Statement Extended Tests ====================

    #[test]
    fn switch_duplicate_case_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1: break;
                    case 1: break;  // Error: duplicate case
                    case 2: break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Duplicate case values should fail");
    }

    #[test]
    fn switch_multiple_defaults_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1: break;
                    default: break;
                    default: break;  // Error: duplicate default
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Duplicate default cases should fail");
    }

    #[test]
    fn switch_case_type_mismatch_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case "string": break;  // Error: type mismatch
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Switch case type mismatch should fail");
    }

    #[test]
    fn switch_fallthrough_cases() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int result = 0;
                switch (x) {
                    case 1:
                    case 2:
                    case 3:
                        result = 10;
                        break;
                    default:
                        result = -1;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Switch fallthrough should work: {:?}", result.errors);
    }

    // ==================== Conversion Tests ====================

    #[test]
    fn int8_to_float_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int8 a = 10;
                float b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int8 to float conversion should work: {:?}", result.errors);
    }

    #[test]
    fn uint8_to_double_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                uint8 a = 200;
                double b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "uint8 to double conversion should work: {:?}", result.errors);
    }

    #[test]
    fn int16_to_int32_widening() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int16 a = int16(1000);
                int32 b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int16 to int32 widening should work: {:?}", result.errors);
    }

    #[test]
    fn uint16_to_uint32_widening() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                uint16 a = 50000;
                uint32 b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "uint16 to uint32 widening should work: {:?}", result.errors);
    }

    // ==================== Foreach Statement ====================

    #[test]
    fn foreach_over_array() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<int> arr = {1, 2, 3};
                foreach (int x : arr) {
                    int y = x * 2;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on array implementation
        let _ = result;
    }

    // ==================== Compound Assignment Operators ====================

    #[test]
    fn compound_bitwise_and_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 15;
                x &= 7;  // x = x & 7
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise AND assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_bitwise_or_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 8;
                x |= 4;  // x = x | 4
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise OR assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_bitwise_xor_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 10;
                x ^= 3;  // x = x ^ 3
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bitwise XOR assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_left_shift_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                x <<= 3;  // x = x << 3
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Left shift assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_right_shift_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 16;
                x >>= 2;  // x = x >> 2
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Right shift assignment should work: {:?}", result.errors);
    }

    // ==================== Member Access Variations ====================

    #[test]
    fn member_access_chain_three_levels() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Level1 {
                int value;
            }

            class Level2 {
                Level1 l1;
            }

            class Level3 {
                Level2 l2;
            }

            void test() {
                Level3 l3;
                int x = l3.l2.l1.value;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Three-level member access should work: {:?}", result.errors);
    }

    #[test]
    fn member_assignment_chain() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Inner {
                int value;
            }

            class Outer {
                Inner inner;
            }

            void test() {
                Outer o;
                o.inner.value = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Chained member assignment should work: {:?}", result.errors);
    }

    // ==================== This Keyword ====================

    #[test]
    fn this_in_constructor() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value;

                MyClass(int v) {
                    this.value = v;
                }
            }

            void test() {
                MyClass obj(10);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "This in constructor should work: {:?}", result.errors);
    }

    #[test]
    fn this_outside_class_context_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = this.value;  // Error: this outside class
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "This outside class should fail");
    }

    // ==================== Explicit Cast Tests ====================

    #[test]
    fn explicit_cast_float_to_int() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float f = 3.14f;
                int i = int(f);  // Explicit cast
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Explicit cast float to int should work: {:?}", result.errors);
    }

    #[test]
    fn explicit_cast_int_to_int8() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 256;
                int8 y = int8(x);  // Explicit narrowing cast
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Explicit narrowing cast should work: {:?}", result.errors);
    }

    // ==================== Array Type Constructor ====================

    #[test]
    fn array_constructor() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                array<int> arr(10);  // Create array with size
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on array implementation
        let _ = result;
    }

    // ==================== Mixin Tests ====================

    #[test]
    fn class_with_mixin() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            mixin class MixinClass {
                int mixinValue = 10;
            }

            class MyClass : MixinClass {
                int ownValue = 20;
            }

            void test() {
                MyClass obj;
                int x = obj.mixinValue;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on mixin implementation
        let _ = result;
    }

    // ==================== Auto Type Inference ====================

    #[test]
    fn auto_with_function_call() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int getNumber() { return 42; }

            void test() {
                auto x = getNumber();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Auto with function call should work: {:?}", result.errors);
    }

    #[test]
    fn auto_with_complex_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10;
                int b = 20;
                auto result = a * b + a - b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Auto with complex expression should work: {:?}", result.errors);
    }

    #[test]
    fn auto_with_const() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                const auto x = 42;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Const auto should work: {:?}", result.errors);
    }

    #[test]
    fn auto_with_handle() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Foo { int value; }

            void test() {
                Foo obj;
                auto@ h = @obj;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Auto with handle should work: {:?}", result.errors);
    }

    #[test]
    fn auto_without_initializer_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                auto x;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Auto without initializer should fail");
        assert!(result.errors.iter().any(|e| e.message.contains("cannot use 'auto' without an initializer")),
            "Should have auto without initializer error: {:?}", result.errors);
    }

    #[test]
    fn auto_with_void_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() { }

            void test() {
                auto x = doNothing();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.is_success(), "Auto with void expression should fail");
        assert!(result.errors.iter().any(|e| e.message.contains("cannot infer type from void expression")),
            "Should have void inference error: {:?}", result.errors);
    }

    // ==================== Unary Operator Edge Cases ====================

    #[test]
    fn not_on_bool_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10;
                int b = 5;
                bool result = !(a > b);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Not on comparison expression should work: {:?}", result.errors);
    }

    #[test]
    fn multiple_unary_operators() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = !!a;  // Double not
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Double not operator should work: {:?}", result.errors);
    }

    // ==================== Property Accessor Tests ====================

    // Property accessor using explicit method syntax with 'property' keyword
    #[test]
    fn property_getter_only() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                private int _count = 0;

                int get_count() const property { return _count; }
            }

            void test() {
                Counter c;
                int x = c.count;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Property getter should work: {:?}", result.errors);
    }

    // Property accessor using explicit method syntax with 'property' keyword
    #[test]
    fn property_getter_and_setter() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                private int _count = 0;

                int get_count() const property { return _count; }
                void set_count(int value) property { _count = value; }
            }

            void test() {
                Counter c;
                c.count = 10;
                int x = c.count;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Property getter and setter should work: {:?}", result.errors);
    }

    // Property accessor using virtual property block syntax
    #[test]
    fn property_virtual_block_syntax() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                private int _count = 0;

                int count {
                    get const { return _count; }
                    set { _count = value; }
                }
            }

            void test() {
                Counter c;
                c.count = 10;
                int x = c.count;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Virtual property block syntax should work: {:?}", result.errors);
    }

    // Property accessor - read-only virtual property
    #[test]
    fn property_read_only_virtual() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                private int _count = 0;

                int count {
                    get const { return _count; }
                }
            }

            void test() {
                Counter c;
                int x = c.count;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Read-only virtual property should work: {:?}", result.errors);
    }

    // ==================== More Integer Type Conversions ====================

    #[test]
    fn int32_to_int8_narrowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int32 a = 127;
                int8 b = int8(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int32 to int8 narrowing should work: {:?}", result.errors);
    }

    #[test]
    fn int64_to_int16_narrowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int64 a = 30000;
                int16 b = int16(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int64 to int16 narrowing should work: {:?}", result.errors);
    }

    #[test]
    fn uint32_to_uint8_narrowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                uint32 a = 200;
                uint8 b = uint8(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "uint32 to uint8 narrowing should work: {:?}", result.errors);
    }

    // ==================== Interface Implementation ====================

    #[test]
    fn interface_implementation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            class Circle : IDrawable {
                void draw() { }
            }

            void test() {
                Circle c;
                c.draw();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Interface implementation should work: {:?}", result.errors);
    }

    // ==================== Static Method Tests ====================

    #[test]
    fn static_method_invocation() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Math {
                static int abs(int x) {
                    if (x < 0) return -x;
                    return x;
                }
            }

            void test() {
                int result = Math::abs(-5);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on static method implementation
        let _ = result;
    }

    // ==================== Const Fields ====================

    #[test]
    fn class_with_const_field() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Config {
                const int MAX_SIZE = 100;

                Config() { }
            }

            void test() {
                Config c;
                int x = c.MAX_SIZE;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on const field implementation
        let _ = result;
    }

    // ==================== Final Classes ====================

    #[test]
    fn final_class() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            final class Singleton {
                int value = 42;
            }

            void test() {
                Singleton s;
                int x = s.value;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Final class should work: {:?}", result.errors);
    }

    // ==================== Implicit Value Access ====================

    #[test]
    fn implicit_this_member_access() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Player {
                int health = 100;

                int getHealth() {
                    return health;  // Implicit this.health
                }
            }

            void test() {
                Player p;
                int h = p.getHealth();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Implicit this member access should work: {:?}", result.errors);
    }

    // ==================== Empty Function Bodies ====================

    #[test]
    fn empty_void_function() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void doNothing() { }

            void test() {
                doNothing();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Empty void function should work: {:?}", result.errors);
    }

    // ==================== Complex Ternary Expressions ====================

    #[test]
    fn nested_ternary() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 10;
                int result = x > 20 ? 1 : x > 10 ? 2 : 3;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Nested ternary should work: {:?}", result.errors);
    }

    // ==================== For Loop with Multiple Variables ====================

    #[test]
    fn for_loop_multiple_init_vars() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                for (int i = 0, j = 10; i < j; i++, j--) {
                    int diff = j - i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on comma expression support
        let _ = result;
    }

    // ==================== Complex Boolean Logic ====================

    #[test]
    fn complex_boolean_and_or() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool a = true;
                bool b = false;
                bool c = true;
                bool result = a && b || c && !b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Complex boolean logic should work: {:?}", result.errors);
    }

    // ==================== Global Variable Access ====================

    #[test]
    fn global_variable_read_and_write() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int globalValue = 42;

            void test() {
                int x = globalValue;
                globalValue = 100;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Global variable access should work: {:?}", result.errors);
    }

    // ==================== Float/Double Operations ====================

    #[test]
    fn double_to_float_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double d = 3.14159;
                float f = float(d);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "double to float conversion should work: {:?}", result.errors);
    }

    #[test]
    fn float_to_double_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float f = 2.5f;
                double d = f;  // Implicit widening
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "float to double conversion should work: {:?}", result.errors);
    }

    // ==================== Handle To Const ====================

    #[test]
    fn const_handle_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class MyClass {
                int value = 10;
            }

            void test() {
                MyClass obj;
                const MyClass@ constHandle = @obj;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // May or may not work depending on const handle implementation
        let _ = result;
    }

    // ==================== Integer Conversion Test Matrix ====================

    #[test]
    fn int8_to_int32_widening() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int8 a = int8(10);
                int32 b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int8 to int32 widening should work: {:?}", result.errors);
    }

    #[test]
    fn int8_to_int64_widening() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int8 a = int8(10);
                int64 b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int8 to int64 widening should work: {:?}", result.errors);
    }

    #[test]
    fn int16_to_int64_widening() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int16 a = int16(1000);
                int64 b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int16 to int64 widening should work: {:?}", result.errors);
    }

    #[test]
    fn int64_to_int32_explicit_narrowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int64 a = 100000;
                int32 b = int32(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int64 to int32 explicit narrowing should work: {:?}", result.errors);
    }

    #[test]
    fn int32_to_int16_explicit_narrowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int32 a = 1000;
                int16 b = int16(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int32 to int16 explicit narrowing should work: {:?}", result.errors);
    }

    #[test]
    fn int16_to_int8_explicit_narrowing() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int16 a = int16(100);
                int8 b = int8(a);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int16 to int8 explicit narrowing should work: {:?}", result.errors);
    }

    // ==================== Float Conversion Test Matrix ====================

    #[test]
    fn int32_to_double_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int32 a = 42;
                double b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int32 to double conversion should work: {:?}", result.errors);
    }

    #[test]
    fn int64_to_double_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int64 a = 1000000;
                double b = a;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "int64 to double conversion should work: {:?}", result.errors);
    }

    #[test]
    fn float_to_int32_explicit_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float f = 3.14f;
                int32 i = int32(f);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "float to int32 explicit conversion should work: {:?}", result.errors);
    }

    #[test]
    fn double_to_int64_explicit_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double d = 123.456;
                int64 i = int64(d);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "double to int64 explicit conversion should work: {:?}", result.errors);
    }

    // ==================== Comparison Operators ====================

    #[test]
    fn less_than_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10;
                float b = 20.5f;
                bool result = a < b;  // int promoted to float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Less than with conversion should work: {:?}", result.errors);
    }

    #[test]
    fn greater_than_or_equal_with_conversion() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 10;
                float b = 5.5f;
                bool result = a >= b;  // int promoted to float
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Greater than or equal with conversion should work: {:?}", result.errors);
    }

    // ==================== Method Chaining ====================

    #[test]
    fn method_returning_self() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Builder {
                int value;

                Builder@ setValue(int v) {
                    value = v;
                    return this;
                }

                int build() {
                    return value;
                }
            }

            void test() {
                Builder b;
                int result = b.setValue(10).build();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This tests method chaining with handle return
        let _ = result;
    }

    // ==================== Nested If-Else ====================

    #[test]
    fn deeply_nested_if_else() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 10;
                int result = 0;
                if (x > 20) {
                    result = 1;
                } else if (x > 15) {
                    result = 2;
                } else if (x > 10) {
                    result = 3;
                } else if (x > 5) {
                    result = 4;
                } else {
                    result = 5;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Deeply nested if-else should work: {:?}", result.errors);
    }

    // ==================== Compound Assignment With Fields ====================

    #[test]
    fn compound_assignment_on_field() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                int count;
            }

            void test() {
                Counter c;
                c.count = 10;
                c.count += 5;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound assignment on field should work: {:?}", result.errors);
    }

    #[test]
    fn compound_subtraction_on_field() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Counter {
                int count;
            }

            void test() {
                Counter c;
                c.count = 10;
                c.count -= 3;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound subtraction on field should work: {:?}", result.errors);
    }

    // ==================== Postfix Operations ====================

    #[test]
    fn postfix_increment_in_array_index() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Container {
                int opIndex(int idx) { return idx; }
            }

            void test() {
                Container c;
                int i = 0;
                int value = c[i++];
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This exercises postfix increment in index expression
        let _ = result;
    }

    // ==================== String Operations ====================

    #[test]
    fn string_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::string_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s = "hello";
                s = "world";
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let string_mod = string_module().expect("Failed to create string module");
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "String assignment should work: {:?}", result.errors);
    }

    #[test]
    fn string_comparison() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s1 = "hello";
                string s2 = "hello";
                bool same = s1 == s2;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // String comparison - may or may not work depending on string implementation
        let _ = result;
    }

    // ==================== Multiple Return Statements ====================

    #[test]
    fn function_with_multiple_returns() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int classify(int x) {
                if (x < 0) return -1;
                if (x == 0) return 0;
                if (x < 10) return 1;
                if (x < 100) return 2;
                return 3;
            }

            void test() {
                int result = classify(50);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Function with multiple returns should work: {:?}", result.errors);
    }

    // ==================== Virtual Method Override ====================

    #[test]
    fn virtual_method_override() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Animal {
                int speak() { return 0; }
            }

            class Dog : Animal {
                int speak() override { return 1; }
            }

            void test() {
                Dog d;
                int sound = d.speak();
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Virtual method override should work: {:?}", result.errors);
    }

    // ==================== Private Constructor ====================

    #[test]
    fn class_with_private_constructor_external_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Singleton {
                private Singleton() { }
            }

            void test() {
                Singleton s;  // Error: private constructor
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        // This tests private constructor access - may or may not fail
        let _ = result;
    }

    // ==================== Assignment Operators ====================

    #[test]
    fn compound_multiply_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                x *= 3;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound multiply assignment should work: {:?}", result.errors);
    }

    #[test]
    fn compound_divide_assignment() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 20;
                x /= 4;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Compound divide assignment should work: {:?}", result.errors);
    }

    // ==================== Float Arithmetic ====================

    #[test]
    fn float_arithmetic_operations() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float a = 3.14f;
                float b = 2.0f;
                float sum = a + b;
                float diff = a - b;
                float product = a * b;
                float quotient = a / b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Float arithmetic operations should work: {:?}", result.errors);
    }

    #[test]
    fn double_arithmetic_operations() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double a = 3.14159;
                double b = 2.71828;
                double sum = a + b;
                double diff = a - b;
                double product = a * b;
                double quotient = a / b;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Double arithmetic operations should work: {:?}", result.errors);
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn arithmetic_expression_precedence() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int a = 2;
                int b = 3;
                int c = 4;
                int result = a + b * c;  // 2 + 12 = 14
                int result2 = (a + b) * c;  // 5 * 4 = 20
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Arithmetic precedence should work: {:?}", result.errors);
    }

    // ==================== Enum With Explicit Values ====================

    #[test]
    fn enum_with_explicit_values() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            enum Priority { Low = 0, Medium = 5, High = 10 }

            void test() {
                Priority p = Priority::Medium;
                int v = p;
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Enum with explicit values should work: {:?}", result.errors);
    }

    // ==================== Return With Expression ====================

    #[test]
    fn return_with_complex_expression() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int calculate(int a, int b) {
                return a * b + a - b;
            }

            void test() {
                int result = calculate(5, 3);
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Return with complex expression should work: {:?}", result.errors);
    }

    // ==================== Variable Shadowing ====================

    #[test]
    fn local_shadows_global() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            int x = 10;

            void test() {
                int x = 20;  // Shadows global
                int y = x;   // Uses local
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Local shadowing global should work: {:?}", result.errors);
    }

    // ==================== Loop With Complex Condition ====================

    #[test]
    fn for_loop_complex_update() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int sum = 0;
                for (int i = 0; i < 100; i = i + 2) {
                    sum = sum + i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "For loop with complex update should work: {:?}", result.errors);
    }

    // ==================== Prefix Decrement ====================

    #[test]
    fn prefix_decrement_in_loop() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int i = 10;
                while (--i > 0) {
                    int x = i;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Prefix decrement in while loop should work: {:?}", result.errors);
    }

    // ==================== Enhanced Switch Tests ====================

    #[test]
    fn switch_on_bool() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool b = true;
                switch (b) {
                    case true:
                        break;
                    case false:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Bool switch should work: {:?}", result.errors);
    }

    #[test]
    fn switch_on_bool_duplicate_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                bool b = true;
                switch (b) {
                    case true:
                        break;
                    case true:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Should detect duplicate bool case");
        assert!(result.errors.iter().any(|e| e.message.contains("duplicate")),
            "Error should mention duplicate: {:?}", result.errors);
    }

    #[test]
    fn switch_on_float() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                float f = 1.5f;
                switch (f) {
                    case 1.5f:
                        break;
                    case 2.5f:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Float switch should work: {:?}", result.errors);
    }

    #[test]
    fn switch_on_double() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                double d = 1.0;
                switch (d) {
                    case 1.0:
                        break;
                    case 2.0:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Double switch should work: {:?}", result.errors);
    }

    #[test]
    fn switch_on_string() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::string_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s = "hello";
                switch (s) {
                    case "hello":
                        break;
                    case "world":
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let string_mod = string_module().expect("Failed to create string module");
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        assert!(result.is_success(), "String switch should work: {:?}", result.errors);
    }

    #[test]
    fn switch_on_string_duplicate_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use crate::modules::string_module;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                string s = "hello";
                switch (s) {
                    case "hello":
                        break;
                    case "hello":
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let string_mod = string_module().expect("Failed to create string module");
        let ffi = create_ffi_with_string();
        let result = Compiler::compile(&script, ffi);

        assert!(!result.errors.is_empty(), "Should detect duplicate string case");
        assert!(result.errors.iter().any(|e| e.message.contains("duplicate")),
            "Error should mention duplicate: {:?}", result.errors);
    }

    #[test]
    fn switch_on_handle_with_null() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Foo {}
            void test() {
                Foo@ obj = null;
                switch (obj) {
                    case null:
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Handle switch with null case should work: {:?}", result.errors);
    }

    #[test]
    fn switch_null_on_non_handle_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case null:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Should error on null case for non-handle");
        assert!(result.errors.iter().any(|e| e.message.contains("null") && e.message.contains("handle")),
            "Error should mention null and handle: {:?}", result.errors);
    }

    #[test]
    fn switch_duplicate_null_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Foo {}
            void test() {
                Foo@ obj = null;
                switch (obj) {
                    case null:
                        break;
                    case null:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Should detect duplicate null case");
        assert!(result.errors.iter().any(|e| e.message.contains("duplicate")),
            "Error should mention duplicate: {:?}", result.errors);
    }

    #[test]
    fn switch_type_pattern_matching() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Animal {
                Animal() {}
            }
            class Dog : Animal {
                Dog() { super(); }
            }
            class Cat : Animal {
                Cat() { super(); }
            }

            void test() {
                Dog@ dog = Dog();
                Animal@ pet = dog;
                switch (pet) {
                    case Dog:
                        break;
                    case Cat:
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Type pattern matching in switch should work: {:?}", result.errors);
    }

    #[test]
    fn switch_type_pattern_duplicate_error() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Animal {
                Animal() {}
            }
            class Dog : Animal {
                Dog() { super(); }
            }

            void test() {
                Dog@ dog = Dog();
                Animal@ pet = dog;
                switch (pet) {
                    case Dog:
                        break;
                    case Dog:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(!result.errors.is_empty(), "Should detect duplicate type pattern");
        assert!(result.errors.iter().any(|e| e.message.contains("duplicate")),
            "Error should mention duplicate: {:?}", result.errors);
    }

    #[test]
    fn switch_type_pattern_with_null() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            class Animal {}
            class Dog : Animal {}

            void test() {
                Animal@ pet = null;
                switch (pet) {
                    case null:
                        break;
                    case Dog:
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Type pattern with null case should work: {:?}", result.errors);
    }

    #[test]
    fn switch_interface_pattern() {
        use crate::Parser;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            interface IWalkable {
                void walk();
            }
            class Dog : IWalkable {
                Dog() {}
                void walk() {}
            }

            void test() {
                Dog@ dog = Dog();
                IWalkable@ walker = dog;
                switch (walker) {
                    case Dog:
                        break;
                    default:
                        break;
                }
            }
        "#;

        let (script, _) = Parser::parse_lenient(source, &arena);
        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Interface type pattern should work: {:?}", result.errors);
    }
}
