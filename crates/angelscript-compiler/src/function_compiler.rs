//! Function compiler for generating bytecode from function bodies.
//!
//! This module provides [`FunctionCompiler`] which compiles a single function's
//! body to bytecode. It handles:
//!
//! - Setting up local scope with parameters
//! - Adding implicit `this` for methods
//! - Compiling function body statements
//! - Verifying return paths for non-void functions
//! - Adding implicit `ReturnVoid` for void functions
//!
//! # Example
//!
//! ```ignore
//! let mut compiler = FunctionCompiler::new(
//!     ctx,
//!     &mut constants,
//!     &func_def,
//!     Some(class_hash), // for methods
//! );
//! compiler.compile_body(&body)?;
//! compiler.verify_returns(span)?;
//! let bytecode = compiler.finish();
//! ```

use angelscript_core::{CompilationError, DataType, FunctionDef, Span, TypeHash};
use angelscript_parser::ast::Block;

use crate::bytecode::{BytecodeChunk, ConstantPool, OpCode};
use crate::context::CompilationContext;
use crate::emit::BytecodeEmitter;
use crate::return_checker::ReturnChecker;
use crate::stmt::StmtCompiler;

type Result<T> = std::result::Result<T, CompilationError>;

/// Compiles a single function body to bytecode.
///
/// Handles parameter setup, body compilation, return verification,
/// and implicit returns for void functions.
pub struct FunctionCompiler<'a, 'ctx, 'pool> {
    /// Compilation context for type lookups and local scope
    ctx: &'a mut CompilationContext<'ctx>,
    /// Bytecode emitter
    emitter: BytecodeEmitter<'pool>,
    /// Function definition (signature)
    def: &'a FunctionDef,
    /// Owner class type (Some for methods, None for global functions)
    owner: Option<TypeHash>,
    /// Whether an explicit return was seen
    has_explicit_return: bool,
}

impl<'a, 'ctx, 'pool> FunctionCompiler<'a, 'ctx, 'pool> {
    /// Create a new function compiler.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Compilation context with type registry
    /// * `constants` - Constant pool for the module
    /// * `def` - Function definition (signature and traits)
    /// * `owner` - Owner class type hash (Some for methods, None for global functions)
    pub fn new(
        ctx: &'a mut CompilationContext<'ctx>,
        constants: &'pool mut ConstantPool,
        def: &'a FunctionDef,
        owner: Option<TypeHash>,
    ) -> Self {
        let emitter = BytecodeEmitter::new(constants);

        Self {
            ctx,
            emitter,
            def,
            owner,
            has_explicit_return: false,
        }
    }

    /// Set up local scope with function parameters.
    ///
    /// For methods, adds implicit `this` parameter first.
    ///
    /// # Arguments
    ///
    /// * `span` - Source location of the function declaration for error reporting
    pub fn setup_parameters(&mut self, span: Span) -> Result<()> {
        self.ctx.begin_function();

        // Add implicit 'this' for methods
        if let Some(owner_hash) = self.owner {
            // Create a handle type for 'this'
            let this_type = DataType::with_handle(owner_hash, self.def.traits.is_const);
            let is_const = self.def.traits.is_const;
            // 'this' is implicit, so use the function span for any errors
            self.ctx
                .declare_param("this".into(), this_type, is_const, span)?;
        }

        // Add explicit parameters
        for param in &self.def.params {
            self.ctx.declare_param(
                param.name.clone(),
                param.data_type,
                param.data_type.is_const,
                span,
            )?;
        }

        Ok(())
    }

    /// Compile the function body.
    ///
    /// # Arguments
    ///
    /// * `body` - The function body (block of statements)
    pub fn compile_body<'ast>(&mut self, body: &Block<'ast>) -> Result<()> {
        let mut stmt_compiler = StmtCompiler::new(
            self.ctx,
            &mut self.emitter,
            self.def.return_type,
            self.owner,
        );

        for stmt in body.stmts {
            stmt_compiler.compile(stmt)?;
        }

        // Check if we saw an explicit return by looking at the bytecode
        // If the last instruction is Return or ReturnVoid, we had an explicit return
        self.has_explicit_return = {
            let checker = ReturnChecker::new();
            checker.all_paths_return(self.emitter.chunk())
        };

        Ok(())
    }

    /// Verify all code paths return a value (for non-void functions).
    ///
    /// # Arguments
    ///
    /// * `span` - Source location for error reporting
    pub fn verify_returns(&self, span: Span) -> Result<()> {
        // Void functions don't need explicit returns
        if self.def.return_type.is_void() {
            return Ok(());
        }

        let checker = ReturnChecker::new();
        if !checker.all_paths_return(self.emitter.chunk()) {
            return Err(CompilationError::Other {
                message: format!(
                    "not all code paths return a value in function '{}'",
                    self.def.name
                ),
                span,
            });
        }

        Ok(())
    }

    /// Finish compilation and return the bytecode.
    ///
    /// For void functions without explicit return, adds implicit `ReturnVoid`.
    pub fn finish(mut self) -> BytecodeChunk {
        // End function scope
        let _scope = self.ctx.end_function();

        // Add implicit return for void functions
        if self.def.return_type.is_void() && !self.has_explicit_return {
            self.emitter.emit(OpCode::ReturnVoid);
        }

        self.emitter.finish()
    }

    /// Get the function definition.
    pub fn def(&self) -> &FunctionDef {
        self.def
    }

    /// Get the owner type hash (for methods).
    pub fn owner(&self) -> Option<TypeHash> {
        self.owner
    }

    /// Check if this is a method (has owner).
    pub fn is_method(&self) -> bool {
        self.owner.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{FunctionTraits, Param, Visibility, primitives};
    use angelscript_registry::SymbolRegistry;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    fn create_void_func_def(name: &str) -> FunctionDef {
        FunctionDef::new(
            TypeHash::from_name(name),
            name.to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        )
    }

    fn create_int_func_def(name: &str) -> FunctionDef {
        FunctionDef::new(
            TypeHash::from_name(name),
            name.to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        )
    }

    fn create_func_with_params(name: &str) -> FunctionDef {
        FunctionDef::new(
            TypeHash::from_name(name),
            name.to_string(),
            vec![],
            vec![
                Param::new("a", DataType::simple(primitives::INT32)),
                Param::new("b", DataType::simple(primitives::INT32)),
            ],
            DataType::simple(primitives::INT32),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        )
    }

    #[test]
    fn function_compiler_creation() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        let def = create_void_func_def("test");

        let compiler = FunctionCompiler::new(&mut ctx, &mut constants, &def, None);
        assert!(!compiler.is_method());
        assert!(compiler.owner().is_none());
    }

    #[test]
    fn method_compiler_creation() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        let def = create_void_func_def("test");
        let class_hash = TypeHash::from_name("MyClass");

        let compiler = FunctionCompiler::new(&mut ctx, &mut constants, &def, Some(class_hash));
        assert!(compiler.is_method());
        assert_eq!(compiler.owner(), Some(class_hash));
    }

    #[test]
    fn setup_parameters_for_function() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        let def = create_func_with_params("add");

        let mut compiler = FunctionCompiler::new(&mut ctx, &mut constants, &def, None);
        compiler.setup_parameters(Span::default()).unwrap();

        // Check parameters were declared
        assert!(ctx.get_local("a").is_some());
        assert!(ctx.get_local("b").is_some());

        // No 'this' for global function
        assert!(ctx.get_local("this").is_none());
    }

    #[test]
    fn setup_parameters_for_method() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        let def = create_void_func_def("getValue");
        let class_hash = TypeHash::from_name("MyClass");

        let mut compiler = FunctionCompiler::new(&mut ctx, &mut constants, &def, Some(class_hash));
        compiler.setup_parameters(Span::default()).unwrap();

        // Check 'this' was declared
        let this_var = ctx.get_local("this");
        assert!(this_var.is_some());
        assert_eq!(this_var.unwrap().data_type.type_hash, class_hash);
        assert!(this_var.unwrap().data_type.is_handle);
    }

    #[test]
    fn void_function_gets_implicit_return() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        let def = create_void_func_def("doNothing");

        let mut compiler = FunctionCompiler::new(&mut ctx, &mut constants, &def, None);
        compiler.setup_parameters(Span::default()).unwrap();

        // Empty body - no explicit return
        let bytecode = compiler.finish();

        // Should have implicit ReturnVoid
        assert!(!bytecode.is_empty());
        assert_eq!(bytecode.read_op(0), Some(OpCode::ReturnVoid));
    }

    #[test]
    fn non_void_function_requires_return() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        let def = create_int_func_def("getValue");

        let mut compiler = FunctionCompiler::new(&mut ctx, &mut constants, &def, None);
        compiler.setup_parameters(Span::default()).unwrap();

        // Empty body - no return
        let result = compiler.verify_returns(Span::default());
        assert!(result.is_err());
    }
}
