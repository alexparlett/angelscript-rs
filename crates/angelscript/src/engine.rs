//! The `Engine` — public API for compiling and executing AngelScript.

use angelscript_compiler::builder::Builder;
use angelscript_compiler::compiler::compile_module;
use angelscript_core::{Diagnostic, QualifiedName, Value};
use angelscript_parser::parser::Parser;
use angelscript_vm::module::{Module, ScriptFunction};
use angelscript_vm::vm::Vm;

/// A compiled script module, ready for execution.
pub struct CompiledModule {
    /// The module name.
    pub name: String,
    /// The VM module containing all functions.
    pub module: Module,
}

/// Errors from the engine.
#[derive(Debug)]
pub enum EngineError {
    /// Compilation failed with diagnostics.
    CompileError(Vec<Diagnostic>),
    /// Runtime error during execution.
    RuntimeError(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::CompileError(diags) => {
                write!(f, "compilation failed:")?;
                for d in diags {
                    write!(f, "\n  {d}")?;
                }
                Ok(())
            }
            EngineError::RuntimeError(msg) => write!(f, "runtime error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// The AngelScript engine — compiles and executes scripts.
pub struct Engine;

impl Engine {
    /// Create a new engine.
    pub fn new() -> Self {
        Engine
    }

    /// Compile a script source into a `CompiledModule`.
    ///
    /// The full pipeline: parse -> build registry -> compile bytecode -> create module.
    pub fn compile(&self, name: &str, source: &str) -> Result<CompiledModule, EngineError> {
        // 1. Parse.
        let mut parser = Parser::new(source);
        let script = parser.parse_script();

        if parser.has_errors() {
            return Err(EngineError::CompileError(parser.diagnostics().to_vec()));
        }

        // 2. Build registry.
        let build_result = Builder::new().build(&[&script]);

        if build_result.has_errors() {
            return Err(EngineError::CompileError(build_result.diagnostics));
        }

        // 3. Compile functions.
        let compile_result = compile_module(&build_result.registry, &[&script]);

        if compile_result.has_errors() {
            return Err(EngineError::CompileError(compile_result.diagnostics));
        }

        // 4. Create VM module.
        let mut module = Module::new();

        for cf in compile_result.functions {
            let param_count = build_result
                .registry
                .get_function(cf.id)
                .map(|fe| fe.signature.param_count() as u16)
                .unwrap_or(0);

            module.add_function(ScriptFunction {
                id: cf.id,
                name: cf.name,
                bytecode: cf.bytecode,
                stack_size: cf.stack_size,
                param_count,
            });
        }

        Ok(CompiledModule {
            name: name.to_string(),
            module,
        })
    }

    /// Call a function by name in a compiled module.
    pub fn call(
        &self,
        compiled: &CompiledModule,
        func_name: &str,
        args: &[Value],
    ) -> Result<Value, EngineError> {
        let name = QualifiedName::global(func_name);
        let mut vm = Vm::new();
        vm.call(&compiled.module, &name, args)
            .map_err(EngineError::RuntimeError)
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
