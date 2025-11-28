//! Unified compiler interface for AngelScript semantic analysis.
//!
//! This module provides a single entry point that orchestrates all compilation passes:
//! - Pass 1: Registration (register all global names)
//! - Pass 2a: Type Compilation (resolve types, fill in type details)
//! - Pass 2b: Function Compilation (compile function bodies to bytecode)
//!
//! # Example
//!
//! ```ignore
//! use angelscript::{parse_lenient, Compiler};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = r#"
//!     class Player {
//!         int health;
//!         Player(int h) { health = h; }
//!     }
//!
//!     void main() {
//!         Player p = Player(100);
//!     }
//! "#;
//!
//! let (script, _) = parse_lenient(source, &arena);
//! let compiled = Compiler::compile(&script);
//!
//! if compiled.errors.is_empty() {
//!     println!("Compiled {} functions", compiled.module.functions.len());
//! }
//! ```

use super::{
    CompiledModule, FunctionCompiler, Registrar, Registry, SemanticError,
    TypeCompiler,
};
use crate::ast::Script;
use rustc_hash::FxHashMap;

/// Result of compiling a complete script.
///
/// This contains all the artifacts from the three compilation passes.
#[derive(Debug)]
pub struct CompilationResult<'src, 'ast> {
    /// The compiled module with all function bytecode
    pub module: CompiledModule,

    /// The complete type registry with all type information
    pub registry: Registry<'src, 'ast>,

    /// Type resolution map (AST span → resolved DataType)
    pub type_map: FxHashMap<crate::lexer::Span, super::DataType>,

    /// All errors encountered across all passes
    pub errors: Vec<SemanticError>,
}

impl<'src, 'ast> CompilationResult<'src, 'ast> {
    /// Check if compilation succeeded (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of functions compiled
    pub fn function_count(&self) -> usize {
        self.module.functions.len()
    }

    /// Get the number of types registered
    pub fn type_count(&self) -> usize {
        self.registry.type_count()
    }
}

/// The unified AngelScript compiler.
///
/// Orchestrates all three compilation passes and returns a complete result.
pub struct Compiler;

impl Compiler {
    /// Compile a complete script through all three passes.
    ///
    /// This performs:
    /// 1. Pass 1: Registration - Register all global names (types, functions, variables)
    /// 2. Pass 2a: Type Compilation - Resolve types and fill in type details
    /// 3. Pass 2b: Function Compilation - Compile all function bodies to bytecode
    ///
    /// Returns a `CompilationResult` containing all compiled artifacts and any errors.
    pub fn compile<'src, 'ast>(script: &'ast Script<'src, 'ast>) -> CompilationResult<'src, 'ast> {
        // Pass 1: Registration
        let registration = Registrar::register(script);

        // Collect Pass 1 errors
        let mut all_errors = registration.errors;

        // Pass 2a: Type Compilation
        let type_compilation = TypeCompiler::compile(script, registration.registry);

        // Collect Pass 2a errors
        all_errors.extend(type_compilation.errors);

        // Pass 2b: Function Compilation
        let mut function_compilation = FunctionCompiler::compile(script, &type_compilation.registry);

        // Collect Pass 2b errors
        all_errors.append(&mut function_compilation.errors);

        CompilationResult {
            module: function_compilation,
            registry: type_compilation.registry,
            type_map: type_compilation.type_map,
            errors: all_errors,
        }
    }

    /// Compile a script and return only the registry (for testing or type-only compilation).
    ///
    /// This performs Pass 1 and Pass 2a only, skipping function body compilation.
    pub fn compile_types<'src, 'ast>(script: &'ast Script<'src, 'ast>) -> TypeCompilationResult<'src, 'ast> {
        // Pass 1: Registration
        let registration = Registrar::register(script);

        // Collect Pass 1 errors
        let mut all_errors = registration.errors;

        // Pass 2a: Type Compilation
        let type_compilation = TypeCompiler::compile(script, registration.registry);

        // Collect Pass 2a errors
        all_errors.extend(type_compilation.errors);

        TypeCompilationResult {
            registry: type_compilation.registry,
            type_map: type_compilation.type_map,
            errors: all_errors,
        }
    }
}

/// Result of type-only compilation (Pass 1 + Pass 2a).
///
/// Useful for tools that only need type information without function bodies.
#[derive(Debug)]
pub struct TypeCompilationResult<'src, 'ast> {
    /// The complete type registry
    pub registry: Registry<'src, 'ast>,

    /// Type resolution map
    pub type_map: FxHashMap<crate::lexer::Span, super::DataType>,

    /// All errors encountered
    pub errors: Vec<SemanticError>,
}

impl<'src, 'ast> TypeCompilationResult<'src, 'ast> {
    /// Check if compilation succeeded (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_lenient;
    use bumpalo::Bump;

    #[test]
    fn compile_empty_script() {
        let arena = Bump::new();
        let (script, _) = parse_lenient("", &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success());
        assert_eq!(result.function_count(), 0);
    }

    #[test]
    fn compile_simple_function() {
        let arena = Bump::new();
        let source = "void main() { }";
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Errors: {:?}", result.errors);
        assert_eq!(result.function_count(), 1);
    }

    #[test]
    fn compile_class_with_constructor() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;
                Player(int h) { }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);

        assert!(result.is_success(), "Errors: {:?}", result.errors);

        // Should have compiled the constructor
        assert!(result.function_count() >= 1, "Expected at least 1 function, got {}", result.function_count());

        // Should have registered the Player type
        assert!(result.registry.lookup_type("Player").is_some());
    }

    #[test]
    fn compile_with_constructor_call() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;
                Player(int h) { }
            }

            void main() {
                Player p = Player(100);
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Errors: {:?}", result.errors);

        // Should have compiled constructor + main
        assert!(result.function_count() >= 2);
    }

    #[test]
    fn compile_types_only() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;
                Player(int h) { }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile_types(&script);
        assert!(result.is_success(), "Errors: {:?}", result.errors);

        // Should have registered the Player type
        assert!(result.registry.lookup_type("Player").is_some());
    }
}
