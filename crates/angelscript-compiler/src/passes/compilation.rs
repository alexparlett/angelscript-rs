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
            Item::Function(func) => self.compile_function(func, None),
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
                        self.compile_function(func, Some(class_hash));
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
        // Pass the class name span for better error messages
        self.compile_auto_generated_methods(class_hash, field_inits.as_ref(), class.name.span);
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
        let base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .and_then(|c| c.base_class);

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
                let mut stmt_compiler = StmtCompiler::new_for_constructor(
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
                let mut stmt_compiler = StmtCompiler::new_for_constructor(
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
                let mut stmt_compiler = StmtCompiler::new_for_constructor(
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
            // No explicit super() call - emit implicit super() if there's a base class
            if let Some(base_class_hash) = base_class {
                // Verify base class has a default constructor
                let base_default_ctor = TypeHash::from_constructor(base_class_hash, &[]);
                if self.ctx.get_function(base_default_ctor).is_none() {
                    // Base class has no default constructor - error
                    let derived_name = self
                        .ctx
                        .get_type(class_hash)
                        .and_then(|e| e.as_class())
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    let base_name = self
                        .ctx
                        .get_type(base_class_hash)
                        .and_then(|e| e.as_class())
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    return Err(CompilationError::NoBaseDefaultConstructor {
                        derived_class: derived_name,
                        base_class: base_name,
                        span,
                    });
                }

                // Emit implicit super() call for base class default constructor
                // Bytecode: GetThis, CallMethod(base_default_ctor)
                emitter.emit_get_this();
                emitter.emit_call_method(base_default_ctor, 0);
            }

            // Step 3: Compile field initializers (fields with explicit init)
            // These run after implicit base class initialization
            field_init::compile_post_base_inits(self.ctx, &mut emitter, class_hash, with_init)?;

            // Compile the rest of the constructor body
            {
                let mut stmt_compiler = StmtCompiler::new_for_constructor(
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
        class_span: Span,
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
                AutoGenKind::DefaultConstructor => self.compile_auto_default_constructor(
                    &func_entry,
                    class_hash,
                    field_inits,
                    class_span,
                ),
                AutoGenKind::CopyConstructor => {
                    self.compile_auto_copy_constructor(&func_entry, class_hash, class_span)
                }
                AutoGenKind::OpAssign => {
                    self.compile_auto_op_assign(&func_entry, class_hash, class_span)
                }
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
        class_span: Span,
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

        // Get base class if it exists
        let base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .and_then(|c| c.base_class);

        // Step 1: Call base class default constructor if needed
        if let Some(base_class_hash) = base_class {
            // Verify base class has a default constructor
            let base_default_ctor = TypeHash::from_constructor(base_class_hash, &[]);
            if self.ctx.get_function(base_default_ctor).is_none() {
                // Base class has no default constructor - can't auto-generate default ctor
                let derived_name = self
                    .ctx
                    .get_type(class_hash)
                    .and_then(|e| e.as_class())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let base_name = self
                    .ctx
                    .get_type(base_class_hash)
                    .and_then(|e| e.as_class())
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(CompilationError::NoBaseDefaultConstructor {
                    derived_class: derived_name,
                    base_class: base_name,
                    span: class_span,
                });
            }

            emitter.emit_get_this();
            emitter.emit_call_method(base_default_ctor, 0);
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
        class_span: Span,
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

        // Get base class if it exists
        let base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .and_then(|c| c.base_class);

        // Step 1: Call base class copy constructor if needed
        if let Some(base_class_hash) = base_class {
            // Copy constructor takes one param: const BaseClass &in
            // The hash is computed from the base class type hash
            let base_copy_ctor = TypeHash::from_constructor(base_class_hash, &[base_class_hash]);

            // Check if base class has a copy constructor - if not, we can't generate one
            if self.ctx.get_function(base_copy_ctor).is_none() {
                return Err(CompilationError::Other {
                    message: "cannot generate copy constructor: base class has no copy constructor"
                        .to_string(),
                    span: class_span,
                });
            }

            emitter.emit_get_this();
            emitter.emit_get_local(1); // 'other' parameter
            emitter.emit_call_method(base_copy_ctor, 1);
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
        class_span: Span,
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

        // Get base class if it exists
        let base_class = self
            .ctx
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .and_then(|c| c.base_class);

        // Step 1: Call base class opAssign if needed
        if let Some(base_class_hash) = base_class {
            // opAssign takes one param: const BaseClass &in
            // opAssign is a method, not a constructor
            let base_op_assign =
                TypeHash::from_method(base_class_hash, "opAssign", &[base_class_hash]);

            // Check if base class has opAssign - if not, we can't generate one
            if self.ctx.get_function(base_op_assign).is_none() {
                return Err(CompilationError::Other {
                    message: "cannot generate opAssign: base class has no opAssign".to_string(),
                    span: class_span,
                });
            }

            emitter.emit_get_this();
            emitter.emit_get_local(1); // 'other' parameter
            emitter.emit_call_method(base_op_assign, 1);
            // opAssign returns this, but we discard it since we'll return our own this
            emitter.emit_pop();
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
    use crate::bytecode::{ConstantPool, OpCode};
    use crate::passes::RegistrationPass;
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

    /// Runs the full compilation pipeline (registration + type completion + compilation)
    fn full_compile(source: &str) -> (CompilationOutput, ConstantPool) {
        let arena = Bump::new();
        let script = Parser::parse(source, &arena).unwrap();

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        // Pass 1: Registration
        let reg_pass = RegistrationPass::new(&mut ctx, UnitId::new(1));
        let reg_output = reg_pass.run(&script);
        assert!(
            reg_output.errors.is_empty(),
            "Registration errors: {:?}",
            reg_output.errors
        );

        // Pass 1b: Type Completion
        let completion_pass = crate::passes::TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let completion_output = completion_pass.run();
        assert!(
            completion_output.errors.is_empty(),
            "Type completion errors: {:?}",
            completion_output.errors
        );

        // Pass 2: Compilation
        let pass = CompilationPass::new(&mut ctx, UnitId::new(1));
        pass.run(&script)
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

    // =========================================================================
    // Implicit super() Call Tests
    // Doc: "If the class is derived from another class and there is no explicit
    //       call to super() in the constructor, the compiler will automatically
    //       insert a call to the default constructor of the base class."
    // =========================================================================

    #[test]
    fn implicit_super_call_in_explicit_constructor() {
        // Test: Derived class constructor without explicit super() should call base default ctor
        let source = r#"
            class Base {}
            class Derived : Base {
                Derived() {}
            }
        "#;

        let (output, constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Derived constructor
        let derived_ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Derived") && f.name.contains("Derived"))
            .expect("Derived constructor not found");

        let bytecode = &derived_ctor.bytecode;

        // First instruction should be GetThis (for calling base ctor)
        assert_eq!(
            bytecode.read_op(0),
            Some(OpCode::GetThis),
            "Expected GetThis as first instruction for implicit super()"
        );

        // Second instruction should be CallMethod (to base default constructor)
        assert_eq!(
            bytecode.read_op(1),
            Some(OpCode::CallMethod),
            "Expected CallMethod for base constructor call"
        );

        // Verify the method hash is for Base constructor
        let base_ctor_hash = TypeHash::from_constructor(TypeHash::from_name("Base"), &[]);
        let constant_idx = bytecode.read_u16(2).expect("Expected constant index");
        let called_hash = constants.get(constant_idx as u32);
        assert!(
            matches!(called_hash, Some(crate::bytecode::Constant::TypeHash(h)) if *h == base_ctor_hash),
            "Expected call to Base default constructor"
        );
    }

    #[test]
    fn no_implicit_super_without_base_class() {
        // Test: Class without base class should not have implicit super()
        let source = r#"
            class Standalone {
                Standalone() {}
            }
        "#;

        let (output, _constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Standalone constructor
        let ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Standalone") && f.name.contains("Standalone"))
            .expect("Constructor not found");

        let bytecode = &ctor.bytecode;

        // Should not have CallMethod (no base class to call)
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            assert_ne!(
                op,
                OpCode::CallMethod,
                "Unexpected CallMethod in class without base"
            );
            offset += 1;
        }
    }

    #[test]
    fn explicit_super_call_no_duplicate_implicit() {
        // Test: When constructor has explicit super(), no implicit super() should be added
        let source = r#"
            class Base {}
            class Derived : Base {
                Derived() {
                    super();
                }
            }
        "#;

        let (output, constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Derived constructor
        let derived_ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Derived") && f.name.contains("Derived"))
            .expect("Derived constructor not found");

        let bytecode = &derived_ctor.bytecode;
        let base_ctor_hash = TypeHash::from_constructor(TypeHash::from_name("Base"), &[]);

        // Count CallMethod instructions calling base constructor
        let mut call_count = 0;
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            if op == OpCode::CallMethod {
                let const_idx = bytecode
                    .read_u16(offset + 1)
                    .expect("Expected constant index");
                if let Some(crate::bytecode::Constant::TypeHash(h)) =
                    constants.get(const_idx as u32)
                    && *h == base_ctor_hash
                {
                    call_count += 1;
                }
                offset += 4; // OpCode + u16 + u8
            } else {
                offset += 1;
            }
        }

        // Should have exactly ONE call to base constructor (explicit, not implicit)
        assert_eq!(
            call_count, 1,
            "Expected exactly 1 call to base constructor, not duplicate"
        );
    }

    #[test]
    fn explicit_super_call_with_args_no_implicit() {
        // Test: When constructor has explicit super(args), no implicit super() should be added
        let source = r#"
            class Base {
                Base(int x) {}
            }
            class Derived : Base {
                Derived() {
                    super(42);
                }
            }
        "#;

        let (output, _constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Derived constructor
        let derived_ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Derived") && f.name.contains("Derived"))
            .expect("Derived constructor not found");

        let bytecode = &derived_ctor.bytecode;

        // Count CallMethod instructions (should only have one for super(42))
        let mut call_method_count = 0;
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            if op == OpCode::CallMethod {
                call_method_count += 1;
                offset += 4;
            } else {
                offset += 1;
            }
        }

        // Should have exactly ONE CallMethod (explicit super(42), no implicit super())
        assert_eq!(
            call_method_count, 1,
            "Expected exactly 1 CallMethod for explicit super(42), no implicit super()"
        );
    }

    // =========================================================================
    // Field Initializer Bytecode Tests
    // Doc: "Members can be initialized at declaration: int a = 10;"
    // =========================================================================

    #[test]
    fn field_initializer_emits_set_field() {
        // Test: Field with initializer should generate GetThis + value + SetField
        let source = r#"
            class Foo {
                int a = 42;
                Foo() {}
            }
        "#;

        let (output, _constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Foo constructor
        let ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Foo") && !f.name.contains("$"))
            .expect("Constructor not found");

        let bytecode = &ctor.bytecode;

        // Look for SetField operation
        let mut found_set_field = false;
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            if op == OpCode::SetField {
                found_set_field = true;
                // Verify field index is 0 (first field)
                let field_idx = bytecode.read_u16(offset + 1).expect("Expected field index");
                assert_eq!(field_idx, 0, "Expected field index 0 for first field");
                break;
            }
            offset += 1;
        }

        assert!(
            found_set_field,
            "Expected SetField instruction for field initializer"
        );
    }

    #[test]
    fn multiple_field_initializers_emit_set_fields_in_order() {
        // Test: Multiple fields with initializers should generate SetField for each
        let source = r#"
            class Foo {
                int a = 10;
                int b = 20;
                int c = 30;
                Foo() {}
            }
        "#;

        let (output, _constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Foo constructor
        let ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Foo") && !f.name.contains("$"))
            .expect("Constructor not found");

        let bytecode = &ctor.bytecode;

        // Collect all SetField instructions and their field indices
        let mut set_field_indices = Vec::new();
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            if op == OpCode::SetField {
                let field_idx = bytecode.read_u16(offset + 1).expect("Expected field index");
                set_field_indices.push(field_idx);
                offset += 3; // OpCode + u16 field index
            } else {
                offset += 1;
            }
        }

        // Should have 3 SetField operations, one for each field
        assert_eq!(
            set_field_indices.len(),
            3,
            "Expected 3 SetField operations for 3 field initializers"
        );

        // Fields should be set in declaration order: a(0), b(1), c(2)
        assert_eq!(
            set_field_indices,
            vec![0, 1, 2],
            "Expected SetField for fields 0, 1, 2 in order"
        );
    }

    #[test]
    fn field_initializer_order_respects_init_grouping() {
        // Test: Fields without init come first, then fields with init
        // class Foo { int a; int b = 10; int c; int d = 20; }
        // Order: a(0), c(2) - no init, then b(1), d(3) - with init
        let source = r#"
            class Foo {
                int a;
                int b = 10;
                int c;
                int d = 20;
                Foo() {}
            }
        "#;

        let (output, _constants) = full_compile(source);
        assert!(
            output.errors.is_empty(),
            "Compilation errors: {:?}",
            output.errors
        );

        // Find the Foo constructor
        let ctor = output
            .functions
            .iter()
            .find(|f| f.name.contains("Foo") && !f.name.contains("$"))
            .expect("Constructor not found");

        let bytecode = &ctor.bytecode;

        // Collect all SetField instructions
        let mut set_field_indices = Vec::new();
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            if op == OpCode::SetField {
                let field_idx = bytecode.read_u16(offset + 1).expect("Expected field index");
                set_field_indices.push(field_idx);
                offset += 3;
            } else {
                offset += 1;
            }
        }

        // Only fields WITH init should have SetField: b(1), d(3)
        // (Fields without init are default-initialized by VM, no SetField)
        assert_eq!(
            set_field_indices.len(),
            2,
            "Expected 2 SetField operations for fields with initializers"
        );
        assert_eq!(
            set_field_indices,
            vec![1, 3],
            "Expected SetField for fields 1 (b) and 3 (d) in declaration order"
        );
    }

    // =========================================================================
    // Auto-Generated Constructor Tests (with stubbed ClassEntry)
    // These tests set up ClassEntry directly with base_class set, then verify
    // the auto-generated methods emit the correct base class calls.
    // =========================================================================

    use crate::bytecode::Constant;
    use angelscript_core::entries::{
        ClassEntry, FunctionEntry, FunctionImpl, FunctionSource, TypeEntry, TypeSource,
    };
    use angelscript_core::{
        AutoGenKind, DataType, FunctionDef, FunctionTraits, Param, Span, TypeKind, Visibility,
    };

    /// Helper to set up Base and Derived classes with inheritance in a context
    fn setup_inheritance_context() -> (SymbolRegistry, TypeHash, TypeHash) {
        let registry = SymbolRegistry::with_primitives();

        let base_hash = TypeHash::from_name("Base");
        let derived_hash = TypeHash::from_name("Derived");

        (registry, base_hash, derived_hash)
    }

    /// Helper to create a function def for auto-generated methods
    fn make_auto_ctor_def(
        hash: TypeHash,
        name: &str,
        params: Vec<Param>,
        return_type: DataType,
        owner: TypeHash,
    ) -> FunctionDef {
        FunctionDef::new(
            hash,
            name.to_string(),
            vec![],
            params,
            return_type,
            Some(owner),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        )
    }

    #[test]
    fn auto_default_constructor_calls_base() {
        let (registry, base_hash, derived_hash) = setup_inheritance_context();
        let mut ctx = CompilationContext::new(&registry);

        // Register Base class default constructor (so derived can call it)
        let base_default_ctor_hash = TypeHash::from_constructor(base_hash, &[]);

        // Register Base class with its default constructor
        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("Base", base_default_ctor_hash);
        ctx.register_type(TypeEntry::Class(base_class)).unwrap();

        // Register Base default constructor function
        let base_default_ctor_def = make_auto_ctor_def(
            base_default_ctor_hash,
            "Base",
            vec![],
            DataType::void(),
            base_hash,
        );
        let base_default_ctor = FunctionEntry::new(
            base_default_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::DefaultConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(base_default_ctor).unwrap();

        // Register auto-generated default constructor for Derived
        let default_ctor_hash = TypeHash::from_constructor(derived_hash, &[]);

        // Register Derived class with base_class set AND the method hash
        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_base(base_hash)
        .with_method("Derived", default_ctor_hash);
        ctx.register_type(TypeEntry::Class(derived_class)).unwrap();

        let default_ctor_def = make_auto_ctor_def(
            default_ctor_hash,
            "Derived",
            vec![],
            DataType::void(),
            derived_hash,
        );
        let default_ctor = FunctionEntry::new(
            default_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::DefaultConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(default_ctor).unwrap();

        // Create compilation pass and compile the auto-generated method
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        // Call compile_auto_generated_methods
        pass.compile_auto_generated_methods(derived_hash, None, Span::default());

        // Check that we got the default constructor compiled
        assert_eq!(pass.compiled_functions.len(), 1);
        let compiled = &pass.compiled_functions[0];
        let bytecode = &compiled.bytecode;

        // First instruction should be GetThis
        assert_eq!(
            bytecode.read_op(0),
            Some(OpCode::GetThis),
            "Expected GetThis as first instruction"
        );

        // Second instruction should be CallMethod
        assert_eq!(
            bytecode.read_op(1),
            Some(OpCode::CallMethod),
            "Expected CallMethod for base constructor call"
        );

        // Verify the method hash is for Base default constructor
        let base_default_ctor = TypeHash::from_constructor(base_hash, &[]);
        let const_idx = bytecode.read_u16(2).unwrap();
        let called_hash = pass.constants.get(const_idx as u32);
        assert!(
            matches!(called_hash, Some(Constant::TypeHash(h)) if *h == base_default_ctor),
            "Expected call to Base default constructor"
        );
    }

    #[test]
    fn auto_copy_constructor_calls_base_copy() {
        let (registry, base_hash, derived_hash) = setup_inheritance_context();
        let mut ctx = CompilationContext::new(&registry);

        // Register Base class copy constructor (so derived can call it)
        let base_copy_ctor_hash = TypeHash::from_constructor(base_hash, &[base_hash]);

        // Register Base class with its copy constructor
        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("Base", base_copy_ctor_hash);
        ctx.register_type(TypeEntry::Class(base_class)).unwrap();

        // Register Base copy constructor function
        let base_copy_ctor_def = make_auto_ctor_def(
            base_copy_ctor_hash,
            "Base",
            vec![Param {
                name: "other".into(),
                data_type: DataType::with_handle(base_hash, true),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            base_hash,
        );
        let base_copy_ctor = FunctionEntry::new(
            base_copy_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::CopyConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(base_copy_ctor).unwrap();

        // Register auto-generated copy constructor for Derived
        let copy_ctor_hash = TypeHash::from_constructor(derived_hash, &[derived_hash]);

        // Register Derived class with base_class set AND the method hash
        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_base(base_hash)
        .with_method("Derived", copy_ctor_hash);
        ctx.register_type(TypeEntry::Class(derived_class)).unwrap();

        let copy_ctor_def = make_auto_ctor_def(
            copy_ctor_hash,
            "Derived",
            vec![Param {
                name: "other".into(),
                data_type: DataType::with_handle(derived_hash, true),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            derived_hash,
        );
        let copy_ctor = FunctionEntry::new(
            copy_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::CopyConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(copy_ctor).unwrap();

        // Create compilation pass and compile the auto-generated method
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        // Call compile_auto_generated_methods
        pass.compile_auto_generated_methods(derived_hash, None, Span::default());

        // Check that we got the copy constructor compiled
        assert_eq!(pass.compiled_functions.len(), 1);
        let compiled = &pass.compiled_functions[0];
        let bytecode = &compiled.bytecode;

        // First instruction should be GetThis
        assert_eq!(
            bytecode.read_op(0),
            Some(OpCode::GetThis),
            "Expected GetThis as first instruction"
        );

        // Second should be GetLocal (slot 1 for 'other')
        assert_eq!(
            bytecode.read_op(1),
            Some(OpCode::GetLocal),
            "Expected GetLocal for 'other' parameter"
        );

        // Third should be CallMethod (after GetLocal's 1-byte operand)
        assert_eq!(
            bytecode.read_op(3),
            Some(OpCode::CallMethod),
            "Expected CallMethod for base copy constructor call"
        );

        // Verify the method hash is for Base copy constructor
        let base_copy_ctor = TypeHash::from_constructor(base_hash, &[base_hash]);
        let const_idx = bytecode.read_u16(4).unwrap();
        let called_hash = pass.constants.get(const_idx as u32);
        assert!(
            matches!(called_hash, Some(Constant::TypeHash(h)) if *h == base_copy_ctor),
            "Expected call to Base copy constructor"
        );
    }

    #[test]
    fn auto_op_assign_calls_base_op_assign() {
        let (registry, base_hash, derived_hash) = setup_inheritance_context();
        let mut ctx = CompilationContext::new(&registry);

        // Register Base class opAssign (so derived can call it)
        let base_op_assign_hash = TypeHash::from_method(base_hash, "opAssign", &[base_hash]);

        // Register Base class with its opAssign
        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("opAssign", base_op_assign_hash);
        ctx.register_type(TypeEntry::Class(base_class)).unwrap();

        // Register Base opAssign function
        let base_op_assign_def = make_auto_ctor_def(
            base_op_assign_hash,
            "opAssign",
            vec![Param {
                name: "other".into(),
                data_type: DataType::with_handle(base_hash, true),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::with_handle(base_hash, false),
            base_hash,
        );
        let base_op_assign = FunctionEntry::new(
            base_op_assign_def,
            FunctionImpl::AutoGenerated(AutoGenKind::OpAssign),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(base_op_assign).unwrap();

        // Register auto-generated opAssign for Derived
        let op_assign_hash = TypeHash::from_method(derived_hash, "opAssign", &[derived_hash]);

        // Register Derived class with base_class set AND the method hash
        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_base(base_hash)
        .with_method("opAssign", op_assign_hash);
        ctx.register_type(TypeEntry::Class(derived_class)).unwrap();

        let op_assign_def = make_auto_ctor_def(
            op_assign_hash,
            "opAssign",
            vec![Param {
                name: "other".into(),
                data_type: DataType::with_handle(derived_hash, true),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::with_handle(derived_hash, false),
            derived_hash,
        );
        let op_assign = FunctionEntry::new(
            op_assign_def,
            FunctionImpl::AutoGenerated(AutoGenKind::OpAssign),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(op_assign).unwrap();

        // Create compilation pass and compile the auto-generated method
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        // Call compile_auto_generated_methods
        pass.compile_auto_generated_methods(derived_hash, None, Span::default());

        // Check that we got the opAssign compiled
        assert_eq!(pass.compiled_functions.len(), 1);
        let compiled = &pass.compiled_functions[0];
        let bytecode = &compiled.bytecode;

        // First instruction should be GetThis
        assert_eq!(
            bytecode.read_op(0),
            Some(OpCode::GetThis),
            "Expected GetThis as first instruction"
        );

        // Second should be GetLocal (slot 1 for 'other')
        assert_eq!(
            bytecode.read_op(1),
            Some(OpCode::GetLocal),
            "Expected GetLocal for 'other' parameter"
        );

        // Third should be CallMethod (after GetLocal's 1-byte operand)
        assert_eq!(
            bytecode.read_op(3),
            Some(OpCode::CallMethod),
            "Expected CallMethod for base opAssign call"
        );

        // Verify the method hash is for Base opAssign
        let base_op_assign = TypeHash::from_method(base_hash, "opAssign", &[base_hash]);
        let const_idx = bytecode.read_u16(4).unwrap();
        let called_hash = pass.constants.get(const_idx as u32);
        assert!(
            matches!(called_hash, Some(Constant::TypeHash(h)) if *h == base_op_assign),
            "Expected call to Base opAssign"
        );

        // Should have Pop after CallMethod (discard base opAssign return)
        // CallMethod has 2-byte index + 1-byte arg count = 3 bytes after opcode
        assert_eq!(
            bytecode.read_op(7),
            Some(OpCode::Pop),
            "Expected Pop to discard base opAssign return value"
        );
    }

    #[test]
    fn auto_default_constructor_no_base_class() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        let class_hash = TypeHash::from_name("Standalone");

        // Register auto-generated default constructor
        let default_ctor_hash = TypeHash::from_constructor(class_hash, &[]);

        // Register class without base class BUT with the method hash
        let class_entry = ClassEntry::new(
            "Standalone",
            vec![],
            "Standalone",
            class_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("Standalone", default_ctor_hash);
        ctx.register_type(TypeEntry::Class(class_entry)).unwrap();

        let default_ctor_def = make_auto_ctor_def(
            default_ctor_hash,
            "Standalone",
            vec![],
            DataType::void(),
            class_hash,
        );
        let default_ctor = FunctionEntry::new(
            default_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::DefaultConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(default_ctor).unwrap();

        // Create compilation pass
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        pass.compile_auto_generated_methods(class_hash, None, Span::default());

        assert_eq!(pass.compiled_functions.len(), 1);
        let bytecode = &pass.compiled_functions[0].bytecode;

        // Should NOT have CallMethod (no base class)
        let mut offset = 0;
        while let Some(op) = bytecode.read_op(offset) {
            assert_ne!(
                op,
                OpCode::CallMethod,
                "Unexpected CallMethod when no base class"
            );
            offset += 1;
        }
    }

    #[test]
    fn auto_default_constructor_errors_when_base_has_no_default_ctor() {
        use angelscript_core::primitives;

        let (registry, base_hash, derived_hash) = setup_inheritance_context();
        let mut ctx = CompilationContext::new(&registry);

        // Register Base class WITHOUT a default constructor (only has parameterized one)
        let base_param_ctor_hash = TypeHash::from_constructor(base_hash, &[primitives::INT32]);

        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("Base", base_param_ctor_hash); // Only has Base(int), no Base()
        ctx.register_type(TypeEntry::Class(base_class)).unwrap();

        // Register the parameterized constructor (NOT a default constructor)
        let base_param_ctor_def = make_auto_ctor_def(
            base_param_ctor_hash,
            "Base",
            vec![Param {
                name: "value".into(),
                data_type: DataType::simple(primitives::INT32),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            base_hash,
        );
        let base_param_ctor = FunctionEntry::new(
            base_param_ctor_def,
            FunctionImpl::Script {
                unit_id: UnitId::new(1),
            },
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(base_param_ctor).unwrap();

        // Register auto-generated default constructor for Derived
        let default_ctor_hash = TypeHash::from_constructor(derived_hash, &[]);

        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_base(base_hash)
        .with_method("Derived", default_ctor_hash);
        ctx.register_type(TypeEntry::Class(derived_class)).unwrap();

        let default_ctor_def = make_auto_ctor_def(
            default_ctor_hash,
            "Derived",
            vec![],
            DataType::void(),
            derived_hash,
        );
        let default_ctor = FunctionEntry::new(
            default_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::DefaultConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(default_ctor).unwrap();

        // Create compilation pass
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        // This should NOT produce any compiled functions because the auto-generation should fail
        pass.compile_auto_generated_methods(derived_hash, None, Span::default());

        // No functions should be compiled (error occurred)
        assert_eq!(
            pass.compiled_functions.len(),
            0,
            "Expected no compiled functions when base has no default constructor"
        );
    }

    #[test]
    fn auto_copy_constructor_errors_when_base_has_no_copy_ctor() {
        let (registry, base_hash, derived_hash) = setup_inheritance_context();
        let mut ctx = CompilationContext::new(&registry);

        // Register Base class WITHOUT a copy constructor
        let base_default_ctor_hash = TypeHash::from_constructor(base_hash, &[]);

        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("Base", base_default_ctor_hash); // Only has Base(), no Base(const Base &in)
        ctx.register_type(TypeEntry::Class(base_class)).unwrap();

        // Register only the default constructor (NOT copy constructor)
        let base_default_ctor_def = make_auto_ctor_def(
            base_default_ctor_hash,
            "Base",
            vec![],
            DataType::void(),
            base_hash,
        );
        let base_default_ctor = FunctionEntry::new(
            base_default_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::DefaultConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(base_default_ctor).unwrap();

        // Register auto-generated copy constructor for Derived
        let copy_ctor_hash = TypeHash::from_constructor(derived_hash, &[derived_hash]);

        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_base(base_hash)
        .with_method("Derived", copy_ctor_hash);
        ctx.register_type(TypeEntry::Class(derived_class)).unwrap();

        let copy_ctor_def = make_auto_ctor_def(
            copy_ctor_hash,
            "Derived",
            vec![Param {
                name: "other".into(),
                data_type: DataType::with_handle(derived_hash, true),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            derived_hash,
        );
        let copy_ctor = FunctionEntry::new(
            copy_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::CopyConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(copy_ctor).unwrap();

        // Create compilation pass
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        // This should NOT produce any compiled functions because base has no copy ctor
        pass.compile_auto_generated_methods(derived_hash, None, Span::default());

        // No functions should be compiled (error suppressed per AngelScript spec)
        assert_eq!(
            pass.compiled_functions.len(),
            0,
            "Expected no compiled functions when base has no copy constructor"
        );
    }

    #[test]
    fn auto_op_assign_errors_when_base_has_no_op_assign() {
        let (registry, base_hash, derived_hash) = setup_inheritance_context();
        let mut ctx = CompilationContext::new(&registry);

        // Register Base class WITHOUT opAssign
        let base_default_ctor_hash = TypeHash::from_constructor(base_hash, &[]);

        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_method("Base", base_default_ctor_hash); // Only has Base(), no opAssign
        ctx.register_type(TypeEntry::Class(base_class)).unwrap();

        // Register only the default constructor (NOT opAssign)
        let base_default_ctor_def = make_auto_ctor_def(
            base_default_ctor_hash,
            "Base",
            vec![],
            DataType::void(),
            base_hash,
        );
        let base_default_ctor = FunctionEntry::new(
            base_default_ctor_def,
            FunctionImpl::AutoGenerated(AutoGenKind::DefaultConstructor),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(base_default_ctor).unwrap();

        // Register auto-generated opAssign for Derived
        let op_assign_hash = TypeHash::from_method(derived_hash, "opAssign", &[derived_hash]);

        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::ScriptObject,
            TypeSource::script(UnitId::new(1), Span::default()),
        )
        .with_base(base_hash)
        .with_method("opAssign", op_assign_hash);
        ctx.register_type(TypeEntry::Class(derived_class)).unwrap();

        let op_assign_def = make_auto_ctor_def(
            op_assign_hash,
            "opAssign",
            vec![Param {
                name: "other".into(),
                data_type: DataType::with_handle(derived_hash, true),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::with_handle(derived_hash, false),
            derived_hash,
        );
        let op_assign = FunctionEntry::new(
            op_assign_def,
            FunctionImpl::AutoGenerated(AutoGenKind::OpAssign),
            FunctionSource::script(Span::default()),
        );
        ctx.register_function(op_assign).unwrap();

        // Create compilation pass
        let constants = ConstantPool::new();
        let mut pass = CompilationPass {
            ctx: &mut ctx,
            unit_id: UnitId::new(1),
            constants,
            compiled_functions: vec![],
            global_inits: vec![],
        };

        // This should NOT produce any compiled functions because base has no opAssign
        pass.compile_auto_generated_methods(derived_hash, None, Span::default());

        // No functions should be compiled (error suppressed per AngelScript spec)
        assert_eq!(
            pass.compiled_functions.len(),
            0,
            "Expected no compiled functions when base has no opAssign"
        );
    }
}
