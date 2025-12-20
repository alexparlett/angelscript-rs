//! Compilation Pass (Pass 2) - Compile function bodies to bytecode.
//!
//! This pass walks the AST after registration and type completion, compiling
//! all function bodies to bytecode. It handles:
//!
//! - Global functions
//! - Class methods (with implicit `this`)
//! - Constructors and destructors
//! - Global variable initializers
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ CompilationPass                                                 │
//! │   - Walks AST items                                             │
//! │   - Enters/exits namespaces                                     │
//! │   - Dispatches to FunctionCompiler for each function            │
//! └─────────────────────────────────────────────────────────────────┘
//!                             │
//!                             ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ FunctionCompiler                                                │
//! │   - Sets up local scope with parameters                         │
//! │   - Uses StmtCompiler for body                                  │
//! │   - Verifies returns                                            │
//! │   - Produces BytecodeChunk                                      │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use angelscript_core::{CompilationError, DataType, FunctionEntry, Span, TypeHash, UnitId};
use angelscript_parser::ast::{
    ClassDecl, ClassMember, FunctionDecl, GlobalVarDecl, Item, NamespaceDecl, Script,
};

use crate::bytecode::{BytecodeChunk, ConstantPool};
use crate::context::CompilationContext;
use crate::field_init::{self, FieldInit};
use crate::function_compiler::FunctionCompiler;
use crate::type_resolver::TypeResolver;

/// Output from the compilation pass.
#[derive(Debug)]
pub struct CompilationOutput {
    /// Unit ID for this compilation.
    pub unit_id: UnitId,
    /// Compiled functions with their bytecode.
    pub functions: Vec<CompiledFunctionEntry>,
    /// Global initializers in declaration order.
    pub global_inits: Vec<GlobalInitEntry>,
    /// Compilation errors.
    pub errors: Vec<CompilationError>,
}

/// A compiled function with its bytecode.
#[derive(Debug)]
pub struct CompiledFunctionEntry {
    /// Function hash (for linking).
    pub hash: TypeHash,
    /// Function name (for debugging).
    pub name: String,
    /// Compiled bytecode.
    pub bytecode: BytecodeChunk,
}

/// A global variable initializer.
#[derive(Debug)]
pub struct GlobalInitEntry {
    /// Global variable hash.
    pub hash: TypeHash,
    /// Variable name.
    pub name: String,
    /// Initializer bytecode (evaluates expression and stores to global).
    pub bytecode: BytecodeChunk,
}

/// Pass 2: Compile function bodies to bytecode.
///
/// This pass walks the AST after registration (Pass 1) and type completion (Pass 1b),
/// compiling all function bodies to bytecode.
pub struct CompilationPass<'a, 'reg> {
    /// Compilation context with type registry and namespace management.
    ctx: &'a mut CompilationContext<'reg>,
    /// Unit ID for this compilation.
    unit_id: UnitId,
    /// Shared constant pool for all functions.
    constants: ConstantPool,
    /// Compiled functions.
    compiled_functions: Vec<CompiledFunctionEntry>,
    /// Global initializers.
    global_inits: Vec<GlobalInitEntry>,
}

impl<'a, 'reg> CompilationPass<'a, 'reg> {
    /// Create a new compilation pass.
    pub fn new(ctx: &'a mut CompilationContext<'reg>, unit_id: UnitId) -> Self {
        Self {
            ctx,
            unit_id,
            constants: ConstantPool::new(),
            compiled_functions: Vec::new(),
            global_inits: Vec::new(),
        }
    }

    /// Run the compilation pass on a script.
    ///
    /// Returns the compilation output and the constant pool.
    pub fn run(mut self, script: &Script<'_>) -> (CompilationOutput, ConstantPool) {
        for item in script.items() {
            self.compile_item(item);
        }

        let output = CompilationOutput {
            unit_id: self.unit_id,
            functions: self.compiled_functions,
            global_inits: self.global_inits,
            errors: self.ctx.take_errors(),
        };

        (output, self.constants)
    }

    // ==========================================================================
    // Item Compilation
    // ==========================================================================

    fn compile_item(&mut self, item: &Item<'_>) {
        match item {
            Item::Namespace(ns) => self.compile_namespace(ns),
            Item::Class(class) => self.compile_class(class),
            Item::Function(func) => self.compile_function(func, None, None),
            Item::GlobalVar(var) => self.compile_global_var(var),

            // These have no code generation in Pass 2
            Item::Interface(_)
            | Item::Enum(_)
            | Item::Typedef(_)
            | Item::Funcdef(_)
            | Item::Mixin(_)
            | Item::Import(_)
            | Item::UsingNamespace(_) => {}
        }
    }

    fn compile_namespace(&mut self, ns: &NamespaceDecl<'_>) {
        let ns_path: Vec<&str> = ns.path.iter().map(|id| id.name).collect();
        let ns_string = ns_path.join("::");

        self.ctx.enter_namespace(&ns_string);

        for item in ns.items {
            self.compile_item(item);
        }

        self.ctx.exit_namespace();
    }

    // ==========================================================================
    // Class Compilation
    // ==========================================================================

    fn compile_class<'ast>(&mut self, class: &ClassDecl<'ast>) {
        let name = class.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let class_hash = TypeHash::from_name(&qualified_name);

        // Collect field initializers for constructors
        let field_inits = match field_init::collect_field_inits(self.ctx, class) {
            Ok((without, with)) => Some((without, with)),
            Err(e) => {
                self.ctx.add_error(e);
                None
            }
        };

        // Compile explicit methods from AST
        for member in class.members {
            match member {
                ClassMember::Method(func) => {
                    // For constructors, pass field initializers
                    if func.is_constructor() {
                        self.compile_constructor(func, class_hash, class, field_inits.as_ref());
                    } else {
                        self.compile_function(func, Some(class_hash), None);
                    }
                }
                ClassMember::Field(_)
                | ClassMember::VirtualProperty(_)
                | ClassMember::Funcdef(_) => {
                    // Fields and virtual properties have no bytecode
                    // (property accessors are registered as methods)
                }
            }
        }

        // Compile auto-generated methods (default ctor, copy ctor, opAssign)
        self.compile_auto_generated_methods(class_hash, field_inits.as_ref());
    }

    fn compile_constructor<'ast>(
        &mut self,
        func: &FunctionDecl<'ast>,
        class_hash: TypeHash,
        class: &ClassDecl<'ast>,
        field_inits: Option<&(Vec<FieldInit<'ast>>, Vec<FieldInit<'ast>>)>,
    ) {
        // Skip functions without bodies (external, abstract, interface)
        let body = match &func.body {
            Some(b) => b,
            None => return,
        };

        // Get the function hash based on registration
        let func_hash = self.compute_function_hash(func, Some(class_hash));

        // Look up the registered function entry
        let func_entry = match self.ctx.get_function(func_hash) {
            Some(entry) => entry.clone(),
            None => {
                self.ctx.add_error(CompilationError::Other {
                    message: format!(
                        "internal error: constructor '{}' not found in registry",
                        func.name.name
                    ),
                    span: func.span,
                });
                return;
            }
        };

        // Skip native/external functions
        if func_entry.is_native() || func_entry.is_external() {
            return;
        }

        // Compile the constructor with field initialization
        match self.compile_constructor_body(
            &func_entry,
            body,
            class_hash,
            class,
            field_inits,
            func.span,
        ) {
            Ok(bytecode) => {
                self.compiled_functions.push(CompiledFunctionEntry {
                    hash: func_hash,
                    name: func_entry.def.name.clone(),
                    bytecode,
                });
            }
            Err(e) => {
                self.ctx.add_error(e);
            }
        }
    }

    // ==========================================================================
    // Function Compilation
    // ==========================================================================

    fn compile_function<'ast>(
        &mut self,
        func: &FunctionDecl<'ast>,
        owner: Option<TypeHash>,
        _field_inits: Option<&(Vec<FieldInit<'ast>>, Vec<FieldInit<'ast>>)>,
    ) {
        // Skip functions without bodies (external, abstract, interface)
        let body = match &func.body {
            Some(b) => b,
            None => return,
        };

        // Get the function hash based on registration
        let func_hash = self.compute_function_hash(func, owner);

        // Look up the registered function entry
        let func_entry = match self.ctx.get_function(func_hash) {
            Some(entry) => entry.clone(),
            None => {
                self.ctx.add_error(CompilationError::Other {
                    message: format!(
                        "internal error: function '{}' not found in registry",
                        func.name.name
                    ),
                    span: func.span,
                });
                return;
            }
        };

        // Skip native/external functions
        if func_entry.is_native() || func_entry.is_external() {
            return;
        }

        // Compile the function
        match self.compile_function_body(&func_entry, body, owner, func.span) {
            Ok(bytecode) => {
                self.compiled_functions.push(CompiledFunctionEntry {
                    hash: func_hash,
                    name: func_entry.def.name.clone(),
                    bytecode,
                });
            }
            Err(e) => {
                self.ctx.add_error(e);
            }
        }
    }

    fn compile_function_body(
        &mut self,
        func_entry: &FunctionEntry,
        body: &angelscript_parser::ast::Block<'_>,
        owner: Option<TypeHash>,
        span: Span,
    ) -> Result<BytecodeChunk, CompilationError> {
        let mut compiler =
            FunctionCompiler::new(self.ctx, &mut self.constants, &func_entry.def, owner);

        // Set up parameters
        compiler.setup_parameters(span)?;

        // Compile body
        compiler.compile_body(body)?;

        // Verify returns for non-void functions
        compiler.verify_returns(span)?;

        // Get bytecode
        Ok(compiler.finish())
    }

    fn compile_constructor_body<'ast>(
        &mut self,
        func_entry: &FunctionEntry,
        body: &angelscript_parser::ast::Block<'ast>,
        class_hash: TypeHash,
        _class: &ClassDecl<'ast>,
        field_inits: Option<&(Vec<FieldInit<'ast>>, Vec<FieldInit<'ast>>)>,
        span: Span,
    ) -> Result<BytecodeChunk, CompilationError> {
        use crate::bytecode::OpCode;
        use crate::emit::BytecodeEmitter;
        use crate::stmt::StmtCompiler;

        // Begin function context
        self.ctx.begin_function();

        // Create emitter
        let mut emitter = BytecodeEmitter::new(&mut self.constants);

        // Add implicit 'this' parameter
        let this_type = DataType::with_handle(class_hash, func_entry.def.traits.is_const);
        self.ctx.declare_param(
            "this".into(),
            this_type,
            func_entry.def.traits.is_const,
            span,
        )?;

        // Add explicit parameters
        for param in &func_entry.def.params {
            self.ctx.declare_param(
                param.name.clone(),
                param.data_type,
                param.data_type.is_const,
                span,
            )?;
        }

        // Check for explicit super() call in constructor body
        let super_call_idx = field_init::find_super_call(body);
        let has_base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.base_class.is_some())
            .unwrap_or(false);

        // Get field initializers
        let (_without_init, with_init) = field_inits
            .map(|(w, wi)| (w.as_slice(), wi.as_slice()))
            .unwrap_or((&[], &[]));

        // AngelScript initialization order:
        // 1. Fields WITHOUT explicit init (default-initialized by VM)
        // 2. Base class initialization (implicit or explicit super())
        // 3. Fields WITH explicit init

        // Step 1: Fields without explicit init are default-initialized by VM
        // (No bytecode needed)

        if let Some(super_idx) = super_call_idx {
            // Explicit super() call exists
            // Compile statements before super()
            {
                let mut stmt_compiler = StmtCompiler::new(
                    self.ctx,
                    &mut emitter,
                    func_entry.def.return_type,
                    Some(class_hash),
                );
                for stmt in &body.stmts[..super_idx] {
                    stmt_compiler.compile(stmt)?;
                }
            }

            // Compile the super() call
            {
                let mut stmt_compiler = StmtCompiler::new(
                    self.ctx,
                    &mut emitter,
                    func_entry.def.return_type,
                    Some(class_hash),
                );
                stmt_compiler.compile(&body.stmts[super_idx])?;
            }

            // Step 3: Compile field initializers (fields with explicit init)
            field_init::compile_post_base_inits(self.ctx, &mut emitter, class_hash, with_init)?;

            // Compile statements after super()
            {
                let mut stmt_compiler = StmtCompiler::new(
                    self.ctx,
                    &mut emitter,
                    func_entry.def.return_type,
                    Some(class_hash),
                );
                for stmt in &body.stmts[super_idx + 1..] {
                    stmt_compiler.compile(stmt)?;
                }
            }
        } else {
            // No explicit super() call
            if has_base_class {
                // TODO: Emit implicit super() call for base class default constructor
                // For now, we just emit field initializers
            }

            // Step 3: Compile field initializers (fields with explicit init)
            // These run after implicit base class initialization
            field_init::compile_post_base_inits(self.ctx, &mut emitter, class_hash, with_init)?;

            // Compile the rest of the constructor body
            {
                let mut stmt_compiler = StmtCompiler::new(
                    self.ctx,
                    &mut emitter,
                    func_entry.def.return_type,
                    Some(class_hash),
                );
                for stmt in body.stmts {
                    stmt_compiler.compile(stmt)?;
                }
            }
        }

        // End function scope
        let _scope = self.ctx.end_function();

        // Add implicit return for void constructors
        if func_entry.def.return_type.is_void() {
            // Check if last instruction is already a return
            let chunk = emitter.chunk();
            let needs_return = chunk.is_empty() || {
                let last_op = chunk.read_op(chunk.len() - 1);
                last_op != Some(OpCode::ReturnVoid) && last_op != Some(OpCode::Return)
            };
            if needs_return {
                emitter.emit(OpCode::ReturnVoid);
            }
        }

        Ok(emitter.finish())
    }

    // ==========================================================================
    // Auto-Generated Method Compilation
    // ==========================================================================

    /// Compile auto-generated methods for a class (default ctor, copy ctor, opAssign).
    ///
    /// These are registered in Pass 1 with `FunctionImpl::AutoGenerated` and need
    /// bytecode generated in Pass 2.
    fn compile_auto_generated_methods<'ast>(
        &mut self,
        class_hash: TypeHash,
        field_inits: Option<&(Vec<FieldInit<'ast>>, Vec<FieldInit<'ast>>)>,
    ) {
        use angelscript_core::AutoGenKind;

        // Get the class entry to find its methods
        let class_entry = match self.ctx.get_type(class_hash) {
            Some(entry) => match entry.as_class() {
                Some(c) => c.clone(),
                None => return,
            },
            None => return,
        };

        // Iterate over all methods and find auto-generated ones
        for method_hash in class_entry.all_methods() {
            let func_entry = match self.ctx.get_function(method_hash) {
                Some(entry) => entry.clone(),
                None => continue,
            };

            // Only compile auto-generated functions
            let auto_kind = match func_entry.auto_gen_kind() {
                Some(kind) => kind,
                None => continue,
            };

            let result = match auto_kind {
                AutoGenKind::DefaultConstructor => {
                    self.compile_auto_default_constructor(&func_entry, class_hash, field_inits)
                }
                AutoGenKind::CopyConstructor => {
                    self.compile_auto_copy_constructor(&func_entry, class_hash)
                }
                AutoGenKind::OpAssign => self.compile_auto_op_assign(&func_entry, class_hash),
            };

            match result {
                Ok(bytecode) => {
                    self.compiled_functions.push(CompiledFunctionEntry {
                        hash: func_entry.def.func_hash,
                        name: func_entry.def.name.clone(),
                        bytecode,
                    });
                }
                Err(e) => {
                    // For copy constructor and opAssign, suppress errors per AngelScript spec
                    // (they simply won't be available if fields can't be copied)
                    if auto_kind == AutoGenKind::DefaultConstructor {
                        self.ctx.add_error(e);
                    }
                }
            }
        }
    }

    /// Compile an auto-generated default constructor.
    ///
    /// Generates bytecode that:
    /// 1. Calls base class default constructor (if base class exists)
    /// 2. Initializes all fields with explicit initializers
    fn compile_auto_default_constructor<'ast>(
        &mut self,
        _func_entry: &FunctionEntry,
        class_hash: TypeHash,
        field_inits: Option<&(Vec<FieldInit<'ast>>, Vec<FieldInit<'ast>>)>,
    ) -> Result<BytecodeChunk, CompilationError> {
        use crate::bytecode::OpCode;
        use crate::emit::BytecodeEmitter;

        // Begin function context
        self.ctx.begin_function();

        // Create emitter
        let mut emitter = BytecodeEmitter::new(&mut self.constants);

        // Add implicit 'this' parameter
        let this_type = DataType::with_handle(class_hash, false);
        self.ctx
            .declare_param("this".into(), this_type, false, Span::default())?;

        // Check if class has a base class
        let has_base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.base_class.is_some())
            .unwrap_or(false);

        // Step 1: Call base class default constructor if needed
        if has_base_class {
            // TODO: Emit call to base class default constructor
            // For now, we skip this - base class init needs super() call support
        }

        // Step 2: Initialize fields with explicit initializers
        if let Some((_without_init, with_init)) = field_inits {
            field_init::compile_post_base_inits(self.ctx, &mut emitter, class_hash, with_init)?;
        }

        // End function scope
        let _scope = self.ctx.end_function();

        // Add implicit return
        emitter.emit(OpCode::ReturnVoid);

        Ok(emitter.finish())
    }

    /// Compile an auto-generated copy constructor.
    ///
    /// Generates bytecode that:
    /// 1. Calls base class copy constructor (if base class exists)
    /// 2. Copies all fields from the 'other' parameter
    fn compile_auto_copy_constructor(
        &mut self,
        func_entry: &FunctionEntry,
        class_hash: TypeHash,
    ) -> Result<BytecodeChunk, CompilationError> {
        use crate::bytecode::OpCode;
        use crate::emit::BytecodeEmitter;

        // Begin function context
        self.ctx.begin_function();

        // Create emitter
        let mut emitter = BytecodeEmitter::new(&mut self.constants);

        // Add implicit 'this' parameter
        let this_type = DataType::with_handle(class_hash, false);
        self.ctx
            .declare_param("this".into(), this_type, false, Span::default())?;

        // Add 'other' parameter (const ClassName &in)
        if let Some(param) = func_entry.def.params.first() {
            self.ctx.declare_param(
                param.name.clone(),
                param.data_type,
                param.data_type.is_const,
                Span::default(),
            )?;
        }

        // Get class properties for field copying
        let properties = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.properties.clone())
            .unwrap_or_default();

        // Check if class has a base class
        let has_base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.base_class.is_some())
            .unwrap_or(false);

        // Step 1: Call base class copy constructor if needed
        if has_base_class {
            // TODO: Emit call to base class copy constructor
        }

        // Step 2: Copy each field from other
        for (field_idx, prop) in properties.iter().enumerate() {
            if !prop.is_direct_field() {
                continue;
            }

            // Load this
            emitter.emit_get_this();

            // Load other.field
            emitter.emit_get_local(1); // 'other' is local 1 (after 'this')
            emitter.emit_get_field(field_idx as u16);

            // Store to this.field
            emitter.emit_set_field(field_idx as u16);
        }

        // End function scope
        let _scope = self.ctx.end_function();

        // Add implicit return
        emitter.emit(OpCode::ReturnVoid);

        Ok(emitter.finish())
    }

    /// Compile an auto-generated opAssign.
    ///
    /// Generates bytecode that:
    /// 1. Calls base class opAssign (if base class exists)
    /// 2. Assigns all fields from the 'other' parameter
    /// 3. Returns this
    fn compile_auto_op_assign(
        &mut self,
        func_entry: &FunctionEntry,
        class_hash: TypeHash,
    ) -> Result<BytecodeChunk, CompilationError> {
        use crate::bytecode::OpCode;
        use crate::emit::BytecodeEmitter;

        // Begin function context
        self.ctx.begin_function();

        // Create emitter
        let mut emitter = BytecodeEmitter::new(&mut self.constants);

        // Add implicit 'this' parameter
        let this_type = DataType::with_handle(class_hash, false);
        self.ctx
            .declare_param("this".into(), this_type, false, Span::default())?;

        // Add 'other' parameter (const ClassName &in)
        if let Some(param) = func_entry.def.params.first() {
            self.ctx.declare_param(
                param.name.clone(),
                param.data_type,
                param.data_type.is_const,
                Span::default(),
            )?;
        }

        // Get class properties for field assignment
        let properties = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.properties.clone())
            .unwrap_or_default();

        // Check if class has a base class
        let has_base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .map(|c| c.base_class.is_some())
            .unwrap_or(false);

        // Step 1: Call base class opAssign if needed
        if has_base_class {
            // TODO: Emit call to base class opAssign
        }

        // Step 2: Assign each field from other
        for (field_idx, prop) in properties.iter().enumerate() {
            if !prop.is_direct_field() {
                continue;
            }

            // Load this
            emitter.emit_get_this();

            // Load other.field
            emitter.emit_get_local(1); // 'other' is local 1 (after 'this')
            emitter.emit_get_field(field_idx as u16);

            // Store to this.field
            emitter.emit_set_field(field_idx as u16);
        }

        // Step 3: Return this
        emitter.emit_get_this();
        emitter.emit(OpCode::Return);

        // End function scope
        let _scope = self.ctx.end_function();

        Ok(emitter.finish())
    }

    fn compute_function_hash(
        &mut self,
        func: &FunctionDecl<'_>,
        owner: Option<TypeHash>,
    ) -> TypeHash {
        let name = func.name.name;
        let qualified_name = if owner.is_some() {
            name.to_string()
        } else {
            self.qualified_name(name)
        };

        // Resolve parameter types to compute hash
        let mut resolver = TypeResolver::new(self.ctx);
        let param_hashes: Vec<TypeHash> = func
            .params
            .iter()
            .filter_map(|p| resolver.resolve(&p.ty.ty).ok().map(|dt| dt.type_hash))
            .collect();

        if func.is_constructor() {
            TypeHash::from_constructor(owner.unwrap(), &param_hashes)
        } else if func.is_destructor {
            // Destructors are hashed as methods named "~" with no parameters
            TypeHash::from_method(owner.unwrap(), "~", &[])
        } else if let Some(owner_hash) = owner {
            TypeHash::from_method(owner_hash, &qualified_name, &param_hashes)
        } else {
            TypeHash::from_function(&qualified_name, &param_hashes)
        }
    }

    // ==========================================================================
    // Global Variable Compilation
    // ==========================================================================

    fn compile_global_var(&mut self, var: &GlobalVarDecl<'_>) {
        // Only compile if there's an initializer
        let init = match &var.init {
            Some(i) => i,
            None => return,
        };

        let name = var.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let var_hash = TypeHash::from_name(&qualified_name);

        // Resolve the variable type
        let mut resolver = TypeResolver::new(self.ctx);
        let var_type = match resolver.resolve(&var.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.ctx.add_error(e);
                return;
            }
        };

        // Compile the initializer
        match self.compile_global_initializer(&qualified_name, var_hash, init, &var_type, var.span)
        {
            Ok(bytecode) => {
                self.global_inits.push(GlobalInitEntry {
                    hash: var_hash,
                    name: qualified_name,
                    bytecode,
                });
            }
            Err(e) => {
                self.ctx.add_error(e);
            }
        }
    }

    fn compile_global_initializer(
        &mut self,
        _name: &str,
        hash: TypeHash,
        init: &angelscript_parser::ast::Expr<'_>,
        var_type: &DataType,
        _span: Span,
    ) -> Result<BytecodeChunk, CompilationError> {
        use crate::bytecode::OpCode;
        use crate::emit::BytecodeEmitter;
        use crate::expr::ExprCompiler;

        // Create a minimal function context for the initializer
        self.ctx.begin_function();

        let mut emitter = BytecodeEmitter::new(&mut self.constants);

        // Compile the initializer expression
        {
            let mut expr_compiler = ExprCompiler::new(self.ctx, &mut emitter, None);
            expr_compiler.check(init, var_type)?;
        }

        // Store to global
        emitter.emit_set_global(hash);

        // Return void
        emitter.emit(OpCode::ReturnVoid);

        self.ctx.end_function();

        Ok(emitter.finish())
    }

    // ==========================================================================
    // Helpers
    // ==========================================================================

    fn qualified_name(&self, name: &str) -> String {
        let ns = self.ctx.current_namespace();
        if ns.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", ns, name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::Parser;
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn parse_and_compile(source: &str) -> CompilationOutput {
        let arena = Bump::new();
        let script = Parser::parse(source, &arena).unwrap();

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        // Note: In real usage, RegistrationPass and TypeCompletionPass run first
        let pass = CompilationPass::new(&mut ctx, UnitId::new(1));
        let (output, _constants) = pass.run(&script);
        output
    }

    #[test]
    fn compile_empty_script() {
        let output = parse_and_compile("");
        assert!(output.functions.is_empty());
        assert!(output.global_inits.is_empty());
        assert!(output.errors.is_empty());
    }

    #[test]
    fn compile_function_without_body() {
        // External function declarations have no body
        let output = parse_and_compile("external void foo();");
        // No functions compiled (no body)
        assert!(output.functions.is_empty());
    }
}
