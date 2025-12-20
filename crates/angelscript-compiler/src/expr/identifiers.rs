//! Identifier expression compilation.
//!
//! Compiles identifier references (variables, globals, 'this').

use angelscript_core::{CompilationError, DataType, Span};
use angelscript_parser::ast::IdentExpr;

use super::{ExprCompiler, Result};
use crate::expr_info::ExprInfo;
use crate::scope::VarLookup;

/// Compile an identifier expression.
pub fn compile_ident<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    ident: &IdentExpr<'ast>,
) -> Result<ExprInfo> {
    let name = ident.ident.name;
    let span = ident.span;

    // Check for 'this' keyword
    if name == "this" {
        return compile_this(compiler, span);
    }

    // Build qualified name if scope is present (e.g., "ns::name")
    let qualified_name = build_qualified_name(ident);

    // First, check local scope (only for unqualified names)
    if ident.scope.is_none()
        && let Some(lookup) = compiler.ctx_mut().get_local_or_capture(name)
    {
        return compile_local(compiler, lookup);
    }

    // Check for globals via CompilationContext
    if let Some(global_hash) = compiler.ctx().resolve_global(&qualified_name) {
        // Get the global entry info before borrowing emitter mutably
        let global_info = compiler
            .ctx()
            .get_global_entry(global_hash)
            .map(|e| (e.data_type, e.is_const));
        if let Some((data_type, is_const)) = global_info {
            compiler.emitter().emit_get_global(global_hash);
            // Use ExprInfo::global to track that this is a global variable
            return Ok(ExprInfo::global(data_type, is_const));
        }
    }

    // Check if we're inside a class and the identifier is a member field (implicit this.field)
    if ident.scope.is_none()
        && let Some(class_hash) = compiler.current_class()
    {
        // Extract field info before mutably borrowing compiler
        let field_info = compiler
            .ctx()
            .get_type(class_hash)
            .and_then(|e| e.as_class())
            .and_then(|class| {
                class
                    .properties
                    .iter()
                    .enumerate()
                    .find(|(_, p)| p.name == name)
                    .map(|(idx, p)| (idx, p.clone()))
            });

        if let Some((field_idx, property)) = field_info {
            // Get 'this' const status from the declared parameter
            let this_is_const = compiler
                .ctx()
                .get_local("this")
                .map(|v| v.is_const)
                .unwrap_or(false);

            // Emit: GetThis, GetField (implicit this.field)
            compiler.emitter().emit_get_this();

            if property.is_direct_field() {
                compiler.emitter().emit_get_field(field_idx as u16);
                return Ok(ExprInfo::member(property.data_type, this_is_const));
            } else if let Some(getter_hash) = property.getter {
                // Virtual property - call getter
                compiler.emitter().emit_call_method(getter_hash, 0);
                return Ok(ExprInfo::rvalue(property.data_type));
            } else {
                // Write-only property - cannot read
                return Err(CompilationError::Other {
                    message: format!("property '{}' is write-only", name),
                    span,
                });
            }
        }
    }

    // Check if it's a function name (for function pointers)
    if let Some(func_hashes) = compiler.ctx().resolve_function(&qualified_name)
        && let Some(&func_hash) = func_hashes.first()
    {
        compiler.emitter().emit_func_ptr(func_hash);
        // TODO: Create proper funcdef type
        return Ok(ExprInfo::rvalue(DataType::simple(func_hash)));
    }

    Err(CompilationError::UndefinedVariable {
        name: qualified_name,
        span,
    })
}

/// Build a qualified name from the identifier expression.
/// E.g., `ns::subns::name` from scope=["ns", "subns"] and ident="name"
pub fn build_qualified_name(ident: &IdentExpr<'_>) -> String {
    match ident.scope {
        Some(scope) if !scope.segments.is_empty() => {
            let mut parts: Vec<&str> = scope.segments.iter().map(|i| i.name).collect();
            parts.push(ident.ident.name);
            parts.join("::")
        }
        _ => ident.ident.name.to_string(),
    }
}

fn compile_this(compiler: &mut ExprCompiler<'_, '_, '_>, span: Span) -> Result<ExprInfo> {
    match compiler.current_class() {
        Some(class_hash) => {
            compiler.emitter().emit_get_this();
            // 'this' is an lvalue that refers to the current object
            // It's effectively a const reference to the object
            let data_type = DataType::with_handle(class_hash, false);
            Ok(ExprInfo::this_ptr(data_type))
        }
        None => Err(CompilationError::ThisOutsideClass { span }),
    }
}

fn compile_local(compiler: &mut ExprCompiler<'_, '_, '_>, lookup: VarLookup) -> Result<ExprInfo> {
    match lookup {
        VarLookup::Local(var) => {
            compiler.emitter().emit_get_local(var.slot);
            // Track that this is a local variable (not safe for ref return)
            Ok(ExprInfo::local(var.data_type, var.is_const))
        }
        VarLookup::Captured(captured) => {
            // For captured variables, emit a closure variable access
            // TODO: Implement closure variable opcodes
            // Captured variables are also local to the enclosing function
            Ok(ExprInfo::local(captured.data_type, captured.is_const))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{ConstantPool, OpCode};
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{TypeHash, primitives};
    use angelscript_registry::SymbolRegistry;

    fn create_test_compiler<'a, 'ctx, 'pool>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter<'pool>,
        current_class: Option<TypeHash>,
    ) -> ExprCompiler<'a, 'ctx, 'pool> {
        ExprCompiler::new(ctx, emitter, current_class)
    }

    fn make_ident_expr(name: &str) -> IdentExpr<'_> {
        use angelscript_parser::ast::Ident;
        IdentExpr {
            scope: None,
            ident: Ident::new(name, Span::new(1, 1, name.len() as u32)),
            type_args: &[],
            span: Span::new(1, 1, name.len() as u32),
        }
    }

    #[test]
    fn compile_local_variable() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable using CompilationContext API
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, None);

        let ident = make_ident_expr("x");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
        assert!(info.is_lvalue);
        assert!(info.is_mutable);

        let chunk = emitter.finish();
        assert_eq!(chunk.read_op(0), Some(OpCode::GetLocal));
    }

    #[test]
    fn compile_const_local() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a const local variable
        let _ = ctx.declare_local(
            "y".to_string(),
            DataType::simple(primitives::DOUBLE),
            true,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, None);

        let ident = make_ident_expr("y");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(info.is_lvalue);
        assert!(!info.is_mutable);
    }

    #[test]
    fn compile_undefined_variable() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, None);

        let ident = make_ident_expr("undefined_var");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(CompilationError::UndefinedVariable { .. })
        ));
    }

    #[test]
    fn compile_this_in_method() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let class_hash = TypeHash::from_name("MyClass");
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, Some(class_hash));

        let ident = make_ident_expr("this");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, class_hash);
        assert!(info.is_lvalue);
        assert!(!info.is_mutable); // 'this' is const

        let chunk = emitter.finish();
        assert_eq!(chunk.read_op(0), Some(OpCode::GetThis));
    }

    #[test]
    fn compile_this_outside_class() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, None);

        let ident = make_ident_expr("this");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(CompilationError::ThisOutsideClass { .. })
        ));
    }

    // =========================================================================
    // Implicit this.field access tests
    // =========================================================================

    use angelscript_core::entries::{ClassEntry, PropertyEntry};
    use angelscript_core::{TypeKind, Visibility};

    fn create_class_with_field(registry: &mut SymbolRegistry) -> TypeHash {
        let class_hash = TypeHash::from_name("TestClass");

        let mut class = ClassEntry::ffi("TestClass", TypeKind::script_object());
        class.properties.push(PropertyEntry::field(
            "x",
            DataType::simple(primitives::INT32),
            Visibility::Public,
        ));
        class.properties.push(PropertyEntry::field(
            "y",
            DataType::simple(primitives::INT32),
            Visibility::Public,
        ));
        registry.register_type(class.into()).unwrap();

        class_hash
    }

    #[test]
    fn compile_implicit_field_access_in_method() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter (as methods do)
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            false, // mutable this
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, Some(class_hash));

        // Access 'x' - should resolve to this.x
        let ident = make_ident_expr("x");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
        assert!(info.is_lvalue);

        let chunk = emitter.finish();
        // Bytecode: GetThis, GetField
        chunk.assert_opcodes(&[OpCode::GetThis, OpCode::GetField]);
    }

    #[test]
    fn compile_implicit_field_access_const_method() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter as const (const method)
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            true, // const this
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, Some(class_hash));

        // Access 'x' - should resolve to this.x
        let ident = make_ident_expr("x");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
        assert!(info.is_lvalue);
        assert!(!info.is_mutable); // const method means field is not mutable

        let chunk = emitter.finish();
        chunk.assert_opcodes(&[OpCode::GetThis, OpCode::GetField]);
    }

    #[test]
    fn compile_implicit_field_local_takes_precedence() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare 'this' parameter
        let _ = ctx.declare_local(
            "this".to_string(),
            DataType::with_handle(class_hash, false),
            false,
            Span::default(),
        );

        // Also declare a local variable named 'x' that shadows the field
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::FLOAT),
            false,
            Span::default(),
        );

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, Some(class_hash));

        // Access 'x' - should resolve to local variable, NOT field
        let ident = make_ident_expr("x");
        let result = compile_ident(&mut compiler, &ident);

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let info = result.unwrap();
        // Should be float (local), not int (field)
        assert_eq!(info.data_type.type_hash, primitives::FLOAT);

        let chunk = emitter.finish();
        // Should use GetLocal, not GetThis + GetField
        chunk.assert_opcodes(&[OpCode::GetLocal]);
    }

    #[test]
    fn compile_implicit_field_not_in_class() {
        let mut registry = SymbolRegistry::with_primitives();
        let class_hash = create_class_with_field(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // No 'this' parameter - not in a class method
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // current_class is None
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter, None);

        // Access 'x' - should fail as undefined variable
        let ident = make_ident_expr("x");
        let result = compile_ident(&mut compiler, &ident);

        // Even though 'x' is a field in the class, we're not in a class method
        // so it should be UndefinedVariable
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::UndefinedVariable { .. }
        ));

        // Suppress unused warning
        let _ = class_hash;
    }
}
