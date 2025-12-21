//! Statement compiler for AngelScript.
//!
//! The [`StmtCompiler`] compiles AST statements to bytecode, handling:
//! - Block statements with proper scoping
//! - Variable declarations with type inference and initialization
//! - Return statements with type checking
//! - If/else control flow
//! - While loops with break/continue support
//!
//! # Example
//!
//! ```ignore
//! let mut compiler = StmtCompiler::new(ctx, emitter, return_type, None);
//!
//! // Compile a statement
//! compiler.compile(&stmt)?;
//! ```

mod block;
mod do_while_stmt;
mod for_stmt;
mod foreach_stmt;
mod if_stmt;
mod return_stmt;
mod switch_stmt;
mod try_catch;
mod var_decl;
mod while_stmt;

use angelscript_core::{CompilationError, DataType, Span, TypeHash, primitives};
use angelscript_parser::ast::Stmt;

use crate::context::CompilationContext;
use crate::emit::BytecodeEmitter;
use crate::expr::ExprCompiler;

type Result<T> = std::result::Result<T, CompilationError>;

/// Compiles statements to bytecode.
///
/// The compiler maintains references to the compilation context and
/// bytecode emitter. It tracks the expected return type for the current
/// function to validate return statements.
pub struct StmtCompiler<'a, 'ctx> {
    /// Compilation context with type registry, namespace info, and local scope
    ctx: &'a mut CompilationContext<'ctx>,
    /// Bytecode emitter
    emitter: &'a mut BytecodeEmitter,
    /// Expected return type for the current function
    return_type: DataType,
    /// Current class type (for 'this' access in methods)
    current_class: Option<TypeHash>,
    /// Whether we're compiling inside a constructor (for super() validation)
    is_constructor: bool,
}

impl<'a, 'ctx> StmtCompiler<'a, 'ctx> {
    /// Create a new statement compiler.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Compilation context with type information and local scope
    /// * `emitter` - Bytecode emitter for output
    /// * `return_type` - Expected return type for the function being compiled
    /// * `current_class` - The class being compiled (for methods)
    pub fn new(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter,
        return_type: DataType,
        current_class: Option<TypeHash>,
    ) -> Self {
        Self {
            ctx,
            emitter,
            return_type,
            current_class,
            is_constructor: false,
        }
    }

    /// Create a new statement compiler for a constructor context.
    ///
    /// This enables super() calls which are only valid in constructors.
    pub fn new_for_constructor(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter,
        return_type: DataType,
        current_class: Option<TypeHash>,
    ) -> Self {
        Self {
            ctx,
            emitter,
            return_type,
            current_class,
            is_constructor: true,
        }
    }

    /// Compile a statement.
    pub fn compile<'ast>(&mut self, stmt: &Stmt<'ast>) -> Result<()> {
        // Set line number for debug info
        let span = stmt.span();
        self.emitter.set_line(span.line);

        match stmt {
            Stmt::Expr(expr_stmt) => self.compile_expr_stmt(expr_stmt),
            Stmt::VarDecl(var_decl) => self.compile_var_decl(var_decl),
            Stmt::Return(ret) => self.compile_return(ret),
            Stmt::Break(brk) => self.compile_break(brk),
            Stmt::Continue(cont) => self.compile_continue(cont),
            Stmt::Block(block) => self.compile_block(block),
            Stmt::If(if_stmt) => self.compile_if(if_stmt),
            Stmt::While(while_stmt) => self.compile_while(while_stmt),

            Stmt::DoWhile(do_while) => self.compile_do_while(do_while),
            Stmt::For(for_stmt) => self.compile_for(for_stmt),
            Stmt::Foreach(foreach) => self.compile_foreach(foreach),
            Stmt::Switch(switch) => self.compile_switch(switch),

            Stmt::TryCatch(try_catch) => self.compile_try_catch(try_catch),
        }
    }

    /// Compile an expression statement.
    ///
    /// Evaluates the expression for its side effects. If the expression
    /// produces a value, it is popped from the stack.
    fn compile_expr_stmt<'ast>(
        &mut self,
        expr_stmt: &angelscript_parser::ast::ExprStmt<'ast>,
    ) -> Result<()> {
        // Empty statement (just a semicolon)
        let Some(expr) = expr_stmt.expr else {
            return Ok(());
        };

        // Compile the expression
        let mut expr_compiler = self.expr_compiler();
        let info = expr_compiler.infer(expr)?;

        // Pop the result if non-void (expression evaluated for side effects only)
        if !info.data_type.is_void() {
            self.emitter.emit_pop();
        }

        Ok(())
    }

    /// Compile a break statement.
    fn compile_break(&mut self, brk: &angelscript_parser::ast::BreakStmt) -> Result<()> {
        self.emitter.emit_break().map_err(|e| match e {
            crate::emit::BreakError::NotInBreakable => CompilationError::Other {
                message: "break statement not inside a loop or switch".to_string(),
                span: brk.span,
            },
            crate::emit::BreakError::NotInLoop => CompilationError::Other {
                message: "break statement not inside a loop".to_string(),
                span: brk.span,
            },
        })
    }

    /// Compile a continue statement.
    fn compile_continue(&mut self, cont: &angelscript_parser::ast::ContinueStmt) -> Result<()> {
        self.emitter.emit_continue().map_err(|e| match e {
            crate::emit::BreakError::NotInLoop => CompilationError::Other {
                message: "continue statement not inside a loop".to_string(),
                span: cont.span,
            },
            crate::emit::BreakError::NotInBreakable => CompilationError::Other {
                message: "continue statement not inside a loop".to_string(),
                span: cont.span,
            },
        })
    }

    /// Create an expression compiler using the current context.
    fn expr_compiler(&mut self) -> ExprCompiler<'_, 'ctx> {
        if self.is_constructor {
            ExprCompiler::new_for_constructor(self.ctx, self.emitter, self.current_class)
        } else {
            ExprCompiler::new(self.ctx, self.emitter, self.current_class)
        }
    }

    // =========================================================================
    // Reference counting helpers
    // =========================================================================

    /// Get the addref behavior function hash for a type.
    ///
    /// For FFI types, returns the registered `behaviors.addref` function.
    /// For script types and funcdefs, returns `TypeHash::SCRIPT_ADDREF` placeholder.
    pub(crate) fn get_addref_behavior(&self, type_hash: TypeHash, span: Span) -> Result<TypeHash> {
        let type_entry = self
            .ctx
            .get_type(type_hash)
            .ok_or_else(|| CompilationError::Other {
                message: format!("unknown type for addref behavior: {:?}", type_hash),
                span,
            })?;

        // Funcdefs use placeholder hash (they're reference counted internally)
        if type_entry.as_funcdef().is_some() {
            return Ok(primitives::SCRIPT_ADDREF);
        }

        let Some(class) = type_entry.as_class() else {
            return Err(CompilationError::Other {
                message: "addref behavior only valid for class types".to_string(),
                span,
            });
        };

        // Script types use placeholder hash
        if class.is_script_object() {
            return Ok(primitives::SCRIPT_ADDREF);
        }

        // FFI types must have addref registered
        class
            .behaviors
            .addref
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' has no addref behavior",
                    type_entry.qualified_name()
                ),
                span,
            })
    }

    /// Get the release behavior function hash for a type.
    ///
    /// For FFI types, returns the registered `behaviors.release` function.
    /// For script types and funcdefs, returns `TypeHash::SCRIPT_RELEASE` placeholder.
    pub(crate) fn get_release_behavior(&self, type_hash: TypeHash, span: Span) -> Result<TypeHash> {
        let type_entry = self
            .ctx
            .get_type(type_hash)
            .ok_or_else(|| CompilationError::Other {
                message: format!("unknown type for release behavior: {:?}", type_hash),
                span,
            })?;

        // Funcdefs use placeholder hash (they're reference counted internally)
        if type_entry.as_funcdef().is_some() {
            return Ok(primitives::SCRIPT_RELEASE);
        }

        let Some(class) = type_entry.as_class() else {
            return Err(CompilationError::Other {
                message: "release behavior only valid for class types".to_string(),
                span,
            });
        };

        // Script types use placeholder hash
        if class.is_script_object() {
            return Ok(primitives::SCRIPT_RELEASE);
        }

        // FFI types must have release registered
        class
            .behaviors
            .release
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' has no release behavior",
                    type_entry.qualified_name()
                ),
                span,
            })
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the compilation context (immutable).
    pub fn ctx(&self) -> &CompilationContext<'ctx> {
        self.ctx
    }

    /// Get the compilation context (mutable).
    pub fn ctx_mut(&mut self) -> &mut CompilationContext<'ctx> {
        self.ctx
    }

    /// Get the bytecode emitter.
    pub fn emitter(&mut self) -> &mut BytecodeEmitter {
        self.emitter
    }

    /// Get the expected return type.
    pub fn return_type(&self) -> &DataType {
        &self.return_type
    }

    /// Get the current class type.
    pub fn current_class(&self) -> Option<TypeHash> {
        self.current_class
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use angelscript_core::{CompilationError, Span, primitives};
    use angelscript_registry::SymbolRegistry;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn stmt_compiler_creation() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let _compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
    }

    #[test]
    fn stmt_compiler_with_class() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let class_hash = TypeHash::from_name("MyClass");
        let compiler =
            StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), Some(class_hash));

        assert_eq!(compiler.current_class(), Some(class_hash));
    }

    #[test]
    fn return_type_accessor() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let return_type = DataType::simple(primitives::INT32);
        let compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        assert_eq!(compiler.return_type().type_hash, primitives::INT32);
    }

    #[test]
    fn empty_expr_stmt() {
        use angelscript_parser::ast::ExprStmt;

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let stmt = ExprStmt {
            expr: None,
            span: Span::default(),
        };
        compiler.compile_expr_stmt(&stmt).unwrap();

        // Empty expression statement emits no bytecode
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn break_outside_loop_error() {
        use angelscript_parser::ast::BreakStmt;

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let brk = BreakStmt {
            span: Span::default(),
        };
        let result = compiler.compile_break(&brk);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::Other { message, .. } => {
                assert!(message.contains("break") && message.contains("loop"));
            }
            other => panic!(
                "Expected Other error for break outside loop, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn continue_outside_loop_error() {
        use angelscript_parser::ast::ContinueStmt;

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let cont = ContinueStmt {
            span: Span::default(),
        };
        let result = compiler.compile_continue(&cont);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::Other { message, .. } => {
                assert!(message.contains("continue") && message.contains("loop"));
            }
            other => panic!(
                "Expected Other error for continue outside loop, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn break_in_loop_succeeds() {
        use crate::bytecode::OpCode;
        use angelscript_parser::ast::BreakStmt;

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Enter a loop context
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let brk = BreakStmt {
            span: Span::default(),
        };
        compiler.compile_break(&brk).unwrap();

        // Break emits a Jump instruction
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 3); // Jump(1) + offset(2)
        assert_eq!(chunk.read_op(0), Some(OpCode::Jump));
    }

    #[test]
    fn continue_in_loop_succeeds() {
        use crate::bytecode::OpCode;
        use angelscript_parser::ast::ContinueStmt;

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Enter a loop context
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let cont = ContinueStmt {
            span: Span::default(),
        };
        compiler.compile_continue(&cont).unwrap();

        // Continue emits a Loop instruction back to loop start
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 3); // Loop(1) + offset(2)
        assert_eq!(chunk.read_op(0), Some(OpCode::Loop));
    }
}
