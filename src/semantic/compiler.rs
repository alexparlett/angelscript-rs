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
    /// The compiled module with all function bytecode (including lambdas)
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

    // ========== Visibility Enforcement Tests ==========

    // Tests for visibility enforcement with explicit `this.member` syntax.

    #[test]
    fn private_field_access_within_class_allowed() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                private int health;

                void setHealth(int h) {
                    this.health = h;  // OK: within same class
                }

                int getHealth() const {
                    return this.health;  // OK: within same class
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn private_field_access_from_outside_class_rejected() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                private int health;
            }

            void main() {
                Player p;
                int h = p.health;  // ERROR: private access
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(!result.is_success(), "Expected error for private field access");
        assert!(result.errors.iter().any(|e|
            e.message.contains("private") && e.message.contains("health")
        ), "Expected 'private' access error, got: {:?}", result.errors);
    }

    #[test]
    fn private_method_access_within_class_allowed() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                private void secretMethod() { }

                void publicMethod() {
                    this.secretMethod();  // OK: within same class
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn private_method_access_from_outside_class_rejected() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                private void secretMethod() { }
            }

            void main() {
                Player p;
                p.secretMethod();  // ERROR: private access
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(!result.is_success(), "Expected error for private method access");
        assert!(result.errors.iter().any(|e|
            e.message.contains("private") && e.message.contains("secretMethod")
        ), "Expected 'private' access error, got: {:?}", result.errors);
    }

    #[test]
    fn protected_field_access_from_derived_class_allowed() {
        let arena = Bump::new();
        let source = r#"
            class Base {
                protected int value;
            }

            class Derived : Base {
                void setValue(int v) {
                    this.value = v;  // OK: protected access from derived class
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn protected_field_access_from_unrelated_class_rejected() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                protected int health;
            }

            void main() {
                Player p;
                int h = p.health;  // ERROR: protected access from outside
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(!result.is_success(), "Expected error for protected field access");
        assert!(result.errors.iter().any(|e|
            e.message.contains("protected") && e.message.contains("health")
        ), "Expected 'protected' access error, got: {:?}", result.errors);
    }

    #[test]
    fn public_field_access_from_anywhere_allowed() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;  // Default is public
            }

            void main() {
                Player p;
                int h = p.health;  // OK: public access
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn public_method_access_from_anywhere_allowed() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                void update() { }  // Default is public
            }

            void main() {
                Player p;
                p.update();  // OK: public access
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn protected_method_access_from_derived_class_allowed() {
        let arena = Bump::new();
        let source = r#"
            class Base {
                protected void helper() { }
            }

            class Derived : Base {
                void doWork() {
                    this.helper();  // OK: protected from derived
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    // =============================================
    // Tests for `this` keyword and implicit member access
    // =============================================

    #[test]
    fn this_keyword_explicit_field_access() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;

                void setHealth(int h) {
                    this.health = h;
                }

                int getHealth() const {
                    return this.health;
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn this_keyword_implicit_field_access() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;

                void setHealth(int h) {
                    health = h;  // Implicit this.health
                }

                int getHealth() const {
                    return health;  // Implicit this.health
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn this_keyword_explicit_method_call() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                void helper() { }

                void update() {
                    this.helper();
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn this_keyword_outside_class_rejected() {
        let arena = Bump::new();
        let source = r#"
            void main() {
                int x = this;  // ERROR: not in a class method
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(!result.is_success(), "Expected error for 'this' outside class");
        assert!(result.errors.iter().any(|e|
            e.message.contains("this") && e.message.contains("class")
        ), "Expected 'this outside class' error, got: {:?}", result.errors);
    }

    #[test]
    fn this_keyword_local_shadows_field() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;

                void setHealth(int health) {
                    this.health = health;  // Parameter shadows field
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn implicit_member_access_inherited_field() {
        let arena = Bump::new();
        let source = r#"
            class Base {
                int value;
            }

            class Derived : Base {
                void setValue(int v) {
                    value = v;  // Implicit this.value (inherited)
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn this_keyword_in_constructor() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;

                Player(int h) {
                    this.health = h;
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn this_keyword_used_in_member_access() {
        let arena = Bump::new();
        let source = r#"
            class Player {
                int health;
                int armor;

                int getTotal() const {
                    return this.health + this.armor;
                }
            }
        "#;
        let (script, _) = parse_lenient(source, &arena);

        let result = Compiler::compile(&script);
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }
}
