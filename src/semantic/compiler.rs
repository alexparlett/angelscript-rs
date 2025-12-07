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
//! use angelscript::{Parser::parse_lenient, Compiler};
//! use bumpalo::Bump;
//! use std::sync::Arc;
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
//! let (script, _) = Parser::parse_lenient(source, &arena);
//! let ffi = Arc::new(FfiRegistryBuilder::new().build().unwrap());
//! let compiled = Compiler::compile(&script, ffi);
//!
//! if compiled.errors.is_empty() {
//!     println!("Compiled {} functions", compiled.module.functions.len());
//! }
//! ```

use std::sync::Arc;

use super::{
    CompiledModule, CompilationContext, FunctionCompiler, Registrar, SemanticError,
    TypeCompiler,
};
use angelscript_parser::ast::Script;
use angelscript_ffi::{FfiRegistry, FfiRegistryBuilder};
use rustc_hash::FxHashMap;

/// Result of compiling a complete script.
///
/// This contains all the artifacts from the three compilation passes.
#[derive(Debug)]
pub struct CompilationResult<'ast> {
    /// The compiled module with all function bytecode (including lambdas)
    pub module: CompiledModule,

    /// The complete compilation context with all type information (FFI + Script)
    pub context: CompilationContext<'ast>,

    /// Type resolution map (AST span → resolved DataType)
    pub type_map: FxHashMap<angelscript_parser::lexer::Span, super::DataType>,

    /// All errors encountered across all passes
    pub errors: Vec<SemanticError>,
}

impl<'ast> CompilationResult<'ast> {
    /// Check if compilation succeeded (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of functions compiled
    pub fn function_count(&self) -> usize {
        self.module.functions.len()
    }

    /// Get the number of types registered (FFI + Script)
    pub fn type_count(&self) -> usize {
        self.context.type_count()
    }
}

/// The unified AngelScript compiler.
///
/// Orchestrates all three compilation passes and returns a complete result.
pub struct Compiler;

impl Compiler {
    /// Compile a complete script through all three passes.
    ///
    /// This is the main compilation entry point. It:
    /// 1. Creates a `CompilationContext` from the FFI registry
    /// 2. Pass 1: Registration - Register all script-defined names
    /// 3. Pass 2a: Type Compilation - Resolve types and fill in type details
    /// 4. Pass 2b: Function Compilation - Compile all function bodies to bytecode
    ///
    /// # Arguments
    /// - `script`: The parsed AST to compile
    /// - `ffi`: An FFI registry containing primitives and any registered FFI types
    ///
    /// # Returns
    /// A `CompilationResult` containing all compiled artifacts and any errors.
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn compile<'ast>(
        script: &'ast Script<'ast>,
        ffi: Arc<FfiRegistry>,
    ) -> CompilationResult<'ast> {
        // Create CompilationContext from FfiRegistry
        let context = CompilationContext::new(ffi);

        // Pass 1: Registration
        let registration = Registrar::register_with_context(script, context);

        // Collect Pass 1 errors
        let mut all_errors = registration.errors;

        // Pass 2a: Type Compilation
        let type_compilation = TypeCompiler::compile(script, registration.context);

        // Collect Pass 2a errors
        all_errors.extend(type_compilation.errors);

        // Pass 2b: Function Compilation
        let mut function_compilation = FunctionCompiler::compile(script, &type_compilation.context);

        // Collect Pass 2b errors
        all_errors.append(&mut function_compilation.errors);

        CompilationResult {
            module: function_compilation,
            context: type_compilation.context,
            type_map: type_compilation.type_map,
            errors: all_errors,
        }
    }

    /// Compile a script and return only the context (for testing or type-only compilation).
    ///
    /// This performs Pass 1 and Pass 2a only, skipping function body compilation.
    pub fn compile_types<'ast>(script: &'ast Script<'ast>) -> TypeCompilationResult<'ast> {
        // Create default FfiRegistry with primitives
        let ffi = Arc::new(FfiRegistryBuilder::new().build().unwrap());
        Self::compile_types_with_ffi(script, ffi)
    }

    /// Compile types only with an FFI registry.
    ///
    /// This performs Pass 1 and Pass 2a only, skipping function body compilation.
    pub fn compile_types_with_ffi<'ast>(
        script: &'ast Script<'ast>,
        ffi: Arc<FfiRegistry>,
    ) -> TypeCompilationResult<'ast> {
        // Create CompilationContext from FfiRegistry
        let context = CompilationContext::new(ffi);

        // Pass 1: Registration
        let registration = Registrar::register_with_context(script, context);

        // Collect Pass 1 errors
        let mut all_errors = registration.errors;

        // Pass 2a: Type Compilation
        let type_compilation = TypeCompiler::compile(script, registration.context);

        // Collect Pass 2a errors
        all_errors.extend(type_compilation.errors);

        TypeCompilationResult {
            context: type_compilation.context,
            type_map: type_compilation.type_map,
            errors: all_errors,
        }
    }
}

/// Result of type-only compilation (Pass 1 + Pass 2a).
///
/// Useful for tools that only need type information without function bodies.
#[derive(Debug)]
pub struct TypeCompilationResult<'ast> {
    /// The complete compilation context with all type information (FFI + Script)
    pub context: CompilationContext<'ast>,

    /// Type resolution map
    pub type_map: FxHashMap<angelscript_parser::lexer::Span, super::DataType>,

    /// All errors encountered
    pub errors: Vec<SemanticError>,
}

impl<'ast> TypeCompilationResult<'ast> {
    /// Check if compilation succeeded (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::Parser;
    use bumpalo::Bump;

    /// Create a default FFI registry with primitives for tests
    fn default_ffi() -> Arc<FfiRegistry> {
        Arc::new(FfiRegistryBuilder::new().build().unwrap())
    }

    #[test]
    fn compile_empty_script() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("", &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success());
        assert_eq!(result.function_count(), 0);
    }

    #[test]
    fn compile_simple_function() {
        let arena = Bump::new();
        let source = "void main() { }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());

        assert!(result.is_success(), "Errors: {:?}", result.errors);

        // Should have compiled the constructor
        assert!(result.function_count() >= 1, "Expected at least 1 function, got {}", result.function_count());

        // Should have registered the Player type
        assert!(result.context.lookup_type("Player").is_some());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile_types(&script);
        assert!(result.is_success(), "Errors: {:?}", result.errors);

        // Should have registered the Player type
        assert!(result.context.lookup_type("Player").is_some());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
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
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    // ==================== Switch Statement Tests ====================

    #[test]
    fn switch_basic_cases_and_default() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int result = 0;
                switch (x) {
                    case 1:
                        result = 10;
                        break;
                    case 2:
                        result = 20;
                        break;
                    default:
                        result = -1;
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn switch_fallthrough_behavior() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int result = 0;
                switch (x) {
                    case 1:
                        result = 10;
                        // No break - falls through to case 2
                    case 2:
                        result = result + 20;
                        break;
                    default:
                        result = -1;
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn switch_multiple_case_labels() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 2;
                int result = 0;
                switch (x) {
                    case 1:
                    case 2:
                    case 3:
                        result = 100;
                        break;
                    default:
                        result = -1;
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn switch_no_default() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 5;
                int result = 0;
                switch (x) {
                    case 1:
                        result = 10;
                        break;
                    case 2:
                        result = 20;
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn switch_nested() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int y = 2;
                int result = 0;
                switch (x) {
                    case 1:
                        switch (y) {
                            case 1:
                                result = 11;
                                break;
                            case 2:
                                result = 12;
                                break;
                        }
                        break;
                    case 2:
                        result = 20;
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn switch_with_enum_values() {
        let arena = Bump::new();
        let source = r#"
            enum Color { RED, GREEN, BLUE }

            void test() {
                int c = Color::GREEN;
                int result = 0;
                switch (c) {
                    case Color::RED:
                        result = 1;
                        break;
                    case Color::GREEN:
                        result = 2;
                        break;
                    case Color::BLUE:
                        result = 3;
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }

    #[test]
    fn switch_duplicate_case_rejected() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
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
        assert!(!result.is_success(), "Expected error for duplicate case values");
        assert!(result.errors.iter().any(|e|
            format!("{:?}", e).contains("duplicate case value")),
            "Expected duplicate case value error, got: {:?}", result.errors);
    }

    #[test]
    fn switch_duplicate_default_rejected() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1:
                        break;
                    default:
                        break;
                    default:
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(!result.is_success(), "Expected error for duplicate default");
        assert!(result.errors.iter().any(|e|
            format!("{:?}", e).contains("one default case")),
            "Expected duplicate default error, got: {:?}", result.errors);
    }

    #[test]
    fn switch_unsupported_type_rejected() {
        let arena = Bump::new();
        // Test that value types (non-handle classes) are rejected
        // We now support: int, bool, float, double, string, enum, and handle types
        // But value-type classes are not supported
        let source = r#"
            class Foo {}
            void test() {
                Foo x;
                switch (x) {
                    case null:
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(!result.is_success(), "Expected error for unsupported switch type");
    }

    #[test]
    fn switch_case_type_mismatch_rejected() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1.5:
                        break;
                }
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(!result.is_success(), "Expected error for type mismatch");
        assert!(result.errors.iter().any(|e|
            format!("{:?}", e).contains("does not match switch type")),
            "Expected type mismatch error, got: {:?}", result.errors);
    }

    #[test]
    fn switch_break_exits_switch() {
        let arena = Bump::new();
        let source = r#"
            void test() {
                int x = 1;
                int result = 0;
                switch (x) {
                    case 1:
                        result = 10;
                        break;
                    case 2:
                        result = 20;
                        break;
                }
                // After switch, result should be 10 (not 30 from fallthrough)
            }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let result = Compiler::compile(&script, default_ffi());
        assert!(result.is_success(), "Expected success, got errors: {:?}", result.errors);
    }
}
