//! Bytecode compiler — compiles AST statements and expressions to bytecode.
//!
//! The compiler works per-function: given a `FunctionDecl` AST node and the
//! populated `SymbolRegistry`, it produces a `CompiledFunction` containing
//! the encoded bytecode and metadata.

pub mod bytecode_builder;
pub mod compile_expr;
pub mod compile_stmt;
pub mod variables;

use angelscript_core::opcode::{ByteCode, OpCode};
use angelscript_core::{DataType, Diagnostic, FunctionId, PrimitiveType, QualifiedName};
use angelscript_parser::ast;

use crate::registry::{FunctionEntry, SymbolRegistry};

use bytecode_builder::{BytecodeBuilder, Label};
use variables::VariableScope;

/// A compiled function ready for the VM.
#[derive(Debug)]
pub struct CompiledFunction {
    /// The function ID in the registry.
    pub id: FunctionId,
    /// The function's qualified name.
    pub name: QualifiedName,
    /// Compiled bytecode.
    pub bytecode: ByteCode,
    /// Stack frame size in dwords.
    pub stack_size: i16,
}

/// Result of an expression compilation.
/// Tracks where the value lives and its type.
#[derive(Debug, Clone)]
pub struct ExprResult {
    /// The type of the expression result.
    pub data_type: DataType,
    /// Whether this is an lvalue (can be assigned to).
    pub is_lvalue: bool,
    /// Whether this is a compile-time constant.
    pub is_constant: bool,
    /// Whether this value is a temporary (compiler-allocated).
    pub is_temporary: bool,
    /// Stack offset in dwords where the value lives.
    pub stack_offset: i16,
    /// Compile-time constant value, if known.
    pub constant_value: Option<ConstantValue>,
}

/// A compile-time constant value.
#[derive(Debug, Clone)]
pub enum ConstantValue {
    Int32(i32),
    Int64(i64),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    Bool(bool),
}

/// Context for break/continue in loops.
#[derive(Debug, Clone)]
struct LoopContext {
    /// Label to jump to for `break`.
    break_label: Label,
    /// Label to jump to for `continue`.
    continue_label: Label,
}

/// Per-function bytecode compiler.
pub struct Compiler<'a> {
    /// The symbol registry from the builder pass.
    registry: &'a SymbolRegistry,
    /// Bytecode builder for the current function.
    bytecode: BytecodeBuilder,
    /// Variable scope/stack allocation tracker.
    variables: VariableScope,
    /// Accumulated diagnostics.
    diagnostics: Vec<Diagnostic>,
    /// Stack of loop contexts for break/continue.
    loop_stack: Vec<LoopContext>,
    /// The current function being compiled.
    current_function: &'a FunctionEntry,
    /// The namespace context for name resolution.
    namespace: Vec<String>,
}

impl<'a> Compiler<'a> {
    /// Create a new compiler for a single function.
    pub fn new(registry: &'a SymbolRegistry, function: &'a FunctionEntry) -> Self {
        Compiler {
            registry,
            bytecode: BytecodeBuilder::new(),
            variables: VariableScope::new(),
            diagnostics: Vec::new(),
            loop_stack: Vec::new(),
            current_function: function,
            namespace: function.name.namespace.clone(),
        }
    }

    /// Compile a function body and produce a `CompiledFunction`.
    pub fn compile(mut self, body: &ast::Stmt) -> CompileResult {
        // Allocate parameters on the stack frame.
        for param in &self.current_function.signature.params {
            self.variables
                .alloc_variable(&param.name, param.data_type.clone());
        }

        // Compile the function body.
        self.compile_statement(body);

        // Ensure we have a return at the end (for void functions).
        if self.current_function.signature.return_type.is_void() {
            self.bytecode.emit_op(OpCode::RET);
        }

        let stack_size = self.variables.frame_size_dwords();
        let bytecode = self.bytecode.finalize_to_bytecode();

        CompileResult {
            function: CompiledFunction {
                id: self.current_function.id,
                name: self.current_function.name.clone(),
                bytecode,
                stack_size,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(msg));
    }

    #[allow(dead_code)]
    fn warning(&mut self, msg: impl Into<String>) {
        self.diagnostics.push(Diagnostic::warning(msg));
    }

    /// Get the primitive type category for choosing the right opcode.
    fn primitive_category(prim: PrimitiveType) -> PrimCategory {
        match prim {
            PrimitiveType::Int8
            | PrimitiveType::Int16
            | PrimitiveType::Int32
            | PrimitiveType::UInt8
            | PrimitiveType::UInt16
            | PrimitiveType::UInt32
            | PrimitiveType::Bool => PrimCategory::Int32,
            PrimitiveType::Int64 | PrimitiveType::UInt64 => PrimCategory::Int64,
            PrimitiveType::Float => PrimCategory::Float,
            PrimitiveType::Double => PrimCategory::Double,
            PrimitiveType::Void => PrimCategory::Void,
        }
    }

    /// Choose the arithmetic opcode variant for a given primitive category.
    fn arith_op(
        category: PrimCategory,
        i32_op: OpCode,
        f_op: OpCode,
        d_op: OpCode,
        i64_op: OpCode,
    ) -> OpCode {
        match category {
            PrimCategory::Int32 => i32_op,
            PrimCategory::Float => f_op,
            PrimCategory::Double => d_op,
            PrimCategory::Int64 => i64_op,
            PrimCategory::Void => i32_op, // shouldn't happen
        }
    }

    /// Choose the comparison opcode for a given primitive category.
    fn cmp_op(category: PrimCategory) -> OpCode {
        match category {
            PrimCategory::Int32 => OpCode::CMPi,
            PrimCategory::Float => OpCode::CMPf,
            PrimCategory::Double => OpCode::CMPd,
            PrimCategory::Int64 => OpCode::CMPi64,
            PrimCategory::Void => OpCode::CMPi,
        }
    }

    /// Emit an implicit type conversion if needed, returning the result type.
    fn emit_conversion(&mut self, from: &DataType, to: &DataType, offset: i16) -> bool {
        let from_prim = match from.as_primitive() {
            Some(p) => p,
            None => return from == to,
        };
        let to_prim = match to.as_primitive() {
            Some(p) => p,
            None => return from == to,
        };

        if from_prim == to_prim {
            return true;
        }

        let from_cat = Self::primitive_category(from_prim);
        let to_cat = Self::primitive_category(to_prim);

        if from_cat == to_cat {
            return true;
        }

        // Emit the conversion instruction.
        let conv_op = match (from_cat, to_cat) {
            (PrimCategory::Int32, PrimCategory::Float) => OpCode::iTOf,
            (PrimCategory::Int32, PrimCategory::Double) => OpCode::iTOd,
            (PrimCategory::Int32, PrimCategory::Int64) => OpCode::iTOi64,
            (PrimCategory::Float, PrimCategory::Int32) => OpCode::fTOi,
            (PrimCategory::Float, PrimCategory::Double) => OpCode::fTOd,
            (PrimCategory::Float, PrimCategory::Int64) => OpCode::fTOi64,
            (PrimCategory::Double, PrimCategory::Int32) => OpCode::dTOi,
            (PrimCategory::Double, PrimCategory::Float) => OpCode::dTOf,
            (PrimCategory::Double, PrimCategory::Int64) => OpCode::dTOi64,
            (PrimCategory::Int64, PrimCategory::Int32) => OpCode::i64TOi,
            (PrimCategory::Int64, PrimCategory::Float) => OpCode::i64TOf,
            (PrimCategory::Int64, PrimCategory::Double) => OpCode::i64TOd,
            _ => return false,
        };

        self.bytecode.emit_w(conv_op, offset);
        true
    }
}

/// Primitive type categories for opcode selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimCategory {
    Int32,
    Int64,
    Float,
    Double,
    Void,
}

/// Result of compiling a function.
pub struct CompileResult {
    pub function: CompiledFunction,
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == angelscript_core::Severity::Error)
    }
}

/// Compile all script functions from the registry.
/// Takes the registry (from builder) and the original ASTs to find function bodies.
pub fn compile_module(registry: &SymbolRegistry, scripts: &[&ast::Script]) -> ModuleCompileResult {
    let mut compiled_functions = Vec::new();
    let mut diagnostics = Vec::new();

    // Walk all scripts to find function bodies and compile them.
    for script in scripts {
        collect_and_compile_functions(
            registry,
            &script.declarations,
            &[],
            &mut compiled_functions,
            &mut diagnostics,
        );
    }

    ModuleCompileResult {
        functions: compiled_functions,
        diagnostics,
    }
}

/// Recursively collect functions with bodies from declarations and compile them.
fn collect_and_compile_functions(
    registry: &SymbolRegistry,
    declarations: &[ast::Declaration],
    namespace: &[String],
    compiled: &mut Vec<CompiledFunction>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for decl in declarations {
        match decl {
            ast::Declaration::Function(f) => {
                compile_function_decl(registry, f, namespace, None, compiled, diagnostics);
            }
            ast::Declaration::Class(c) => {
                let class_name = QualifiedName::new(namespace.to_vec(), &c.name);
                for member in &c.members {
                    if let ast::ClassMember::Function(f) = member {
                        compile_function_decl(
                            registry,
                            f,
                            namespace,
                            Some(&class_name),
                            compiled,
                            diagnostics,
                        );
                    }
                }
            }
            ast::Declaration::Namespace(n) => {
                let mut new_ns = namespace.to_vec();
                new_ns.extend(n.name.iter().cloned());
                collect_and_compile_functions(
                    registry,
                    &n.declarations,
                    &new_ns,
                    compiled,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
}

/// Compile a single function declaration if it has a body.
fn compile_function_decl(
    registry: &SymbolRegistry,
    func: &ast::FunctionDecl,
    namespace: &[String],
    owner: Option<&QualifiedName>,
    compiled: &mut Vec<CompiledFunction>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let body = match &func.body {
        Some(body) => body,
        None => return, // No body (forward declaration or interface method).
    };

    let func_name = if func.is_destructor {
        format!("~{}", func.name)
    } else {
        func.name.clone()
    };

    let name = if let Some(owner_name) = owner {
        let mut ns = owner_name.namespace.clone();
        ns.push(owner_name.name.clone());
        QualifiedName::new(ns, &func_name)
    } else {
        QualifiedName::new(namespace.to_vec(), &func_name)
    };

    // Find the function entry in the registry.
    let func_ids = registry.get_functions_by_name(&name);
    if func_ids.is_empty() {
        diagnostics.push(Diagnostic::error(format!(
            "function '{}' not found in registry",
            name
        )));
        return;
    }

    // For now, take the first matching function.
    // TODO: match by parameter types for overloads.
    let func_entry = match registry.get_function(func_ids[0]) {
        Some(entry) => entry,
        None => return,
    };

    let compiler = Compiler::new(registry, func_entry);
    let result = compiler.compile(body);

    diagnostics.extend(result.diagnostics);
    compiled.push(result.function);
}

/// Result of compiling an entire module.
pub struct ModuleCompileResult {
    pub functions: Vec<CompiledFunction>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ModuleCompileResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == angelscript_core::Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Builder;
    use angelscript_parser::parser::Parser;

    fn parse(source: &str) -> ast::Script {
        let mut parser = Parser::new(source);
        let script = parser.parse_script();
        assert!(
            !parser.has_errors(),
            "parse errors: {:?}",
            parser.diagnostics()
        );
        script
    }

    fn build_and_compile(source: &str) -> ModuleCompileResult {
        let script = parse(source);
        let build_result = Builder::new().build(&[&script]);
        assert!(
            !build_result.has_errors(),
            "build errors: {:?}",
            build_result.diagnostics
        );
        compile_module(&build_result.registry, &[&script])
    }

    #[test]
    fn compile_empty_function() {
        let result = build_and_compile("void main() {}");
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
        assert_eq!(result.functions.len(), 1);
        assert!(!result.functions[0].bytecode.is_empty());
    }

    #[test]
    fn compile_return_void() {
        let result = build_and_compile("void foo() { return; }");
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
        assert_eq!(result.functions.len(), 1);
    }

    #[test]
    fn compile_return_int() {
        let result = build_and_compile("int answer() { return 42; }");
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
        assert_eq!(result.functions.len(), 1);
    }

    #[test]
    fn compile_var_declaration() {
        let result = build_and_compile(
            r#"
            void foo() {
                int x = 10;
                float y = 3.14f;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_arithmetic() {
        let result = build_and_compile(
            r#"
            int calc() {
                int a = 10;
                int b = 20;
                return a + b;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_if_else() {
        let result = build_and_compile(
            r#"
            int abs(int x) {
                if (x < 0) {
                    return -x;
                } else {
                    return x;
                }
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_while_loop() {
        let result = build_and_compile(
            r#"
            int sum() {
                int i = 0;
                int s = 0;
                while (i < 10) {
                    s = s + i;
                    i = i + 1;
                }
                return s;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_for_loop() {
        let result = build_and_compile(
            r#"
            int sum() {
                int s = 0;
                for (int i = 0; i < 10; i = i + 1) {
                    s = s + i;
                }
                return s;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_break_continue() {
        let result = build_and_compile(
            r#"
            void loop_test() {
                int i = 0;
                while (true) {
                    if (i > 5) { break; }
                    i = i + 1;
                    continue;
                }
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_nested_scopes() {
        let result = build_and_compile(
            r#"
            void foo() {
                int x = 1;
                {
                    int y = 2;
                    int z = x + y;
                }
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_do_while() {
        let result = build_and_compile(
            r#"
            void foo() {
                int i = 0;
                do {
                    i = i + 1;
                } while (i < 10);
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_ternary() {
        let result = build_and_compile(
            r#"
            int max(int a, int b) {
                return a > b ? a : b;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_class_method() {
        let result = build_and_compile(
            r#"
            class Foo {
                int getValue() { return 42; }
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
        assert_eq!(result.functions.len(), 1);
    }

    #[test]
    fn compile_multiple_functions() {
        let result = build_and_compile(
            r#"
            int add(int a, int b) { return a + b; }
            int sub(int a, int b) { return a - b; }
            void main() {
                int x = 10;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
        assert_eq!(result.functions.len(), 3);
    }

    #[test]
    fn compile_unary_negation() {
        let result = build_and_compile(
            r#"
            int neg(int x) { return -x; }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_logical_not() {
        let result = build_and_compile(
            r#"
            bool invert(bool x) { return !x; }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_float_arithmetic() {
        let result = build_and_compile(
            r#"
            float calc() {
                float a = 1.5f;
                float b = 2.5f;
                return a * b;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn compile_double_arithmetic() {
        let result = build_and_compile(
            r#"
            double calc() {
                double a = 1.5;
                double b = 2.5;
                return a + b;
            }
            "#,
        );
        assert!(
            !result.has_errors(),
            "compile errors: {:?}",
            result.diagnostics
        );
    }
}
