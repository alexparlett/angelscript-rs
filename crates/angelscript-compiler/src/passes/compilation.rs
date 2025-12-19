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

    fn compile_class(&mut self, class: &ClassDecl<'_>) {
        let name = class.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let class_hash = TypeHash::from_name(&qualified_name);

        for member in class.members {
            match member {
                ClassMember::Method(func) => {
                    self.compile_function(func, Some(class_hash));
                }
                ClassMember::Field(_)
                | ClassMember::VirtualProperty(_)
                | ClassMember::Funcdef(_) => {
                    // Fields and virtual properties have no bytecode
                    // (property accessors are registered as methods)
                }
            }
        }
    }

    // ==========================================================================
    // Function Compilation
    // ==========================================================================

    fn compile_function(&mut self, func: &FunctionDecl<'_>, owner: Option<TypeHash>) {
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
        compiler.setup_parameters()?;

        // Compile body
        compiler.compile_body(body)?;

        // Verify returns for non-void functions
        compiler.verify_returns(span)?;

        // Get bytecode
        Ok(compiler.finish())
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
