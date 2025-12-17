//! Init list expression compilation.
//!
//! Handles `{elem1, elem2, ...}` syntax for initializer lists.
//!
//! ## Usage
//!
//! Init lists require:
//! 1. An expected type (from context, like `array<int> arr = {...}`)
//! 2. The target type must have a list factory or list construct behavior
//!
//! If either condition is not met, compilation fails with an error.

use angelscript_core::{CompilationError, DataType, Span};
use angelscript_parser::ast::{InitElement, InitListExpr};

use super::{ExprCompiler, Result};
use crate::expr_info::ExprInfo;
use crate::type_resolver::TypeResolver;

/// Compile an init list expression: `{elem1, elem2, ...}`
///
/// Init lists require an expected type to determine the target. Without
/// a target type, the compiler cannot determine how to construct the value.
///
/// # Parameters
///
/// * `expected` - The expected type from context (e.g., variable declaration)
///
/// # Returns
///
/// * `Ok(ExprInfo)` - If the init list is valid for the expected type
/// * `Err(CompilationError)` - If no expected type, or type doesn't support init lists
pub fn compile_init_list<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &InitListExpr<'ast>,
    expected: Option<&DataType>,
) -> Result<ExprInfo> {
    let span = expr.span;

    // 1. Determine target type from explicit annotation or expected type
    let target_type = if let Some(ty) = &expr.ty {
        // Explicit type annotation: `array<int>{1, 2, 3}`
        let mut resolver = TypeResolver::new(compiler.ctx_mut());
        resolver.resolve(ty)?
    } else if let Some(expected) = expected {
        // Inferred from context: `array<int> arr = {1, 2, 3}`
        *expected
    } else {
        // No type information available
        return Err(CompilationError::TypeMismatch {
            message:
                "init list requires a target type (use explicit type or assign to typed variable)"
                    .to_string(),
            span,
        });
    };

    // 2. Look up the type's list initialization behavior
    let list_init_func = get_list_init_behavior(compiler, target_type.type_hash, span)?;

    // 3. Get element type from the target type's template arguments
    let element_type = get_element_type(compiler, target_type.type_hash, span)?;

    // 4. Compile each element with type checking
    for element in expr.elements.iter() {
        compile_element(compiler, element, &element_type)?;
    }

    // 5. Emit the list factory/construct call
    let element_count = expr.elements.len() as u8;
    compiler.emitter().emit_call(list_init_func, element_count);

    Ok(ExprInfo::rvalue(target_type))
}

/// Get the list initialization behavior for a type.
///
/// Returns the function hash for list_factory or list_construct.
fn get_list_init_behavior(
    compiler: &ExprCompiler<'_, '_, '_>,
    type_hash: angelscript_core::TypeHash,
    span: Span,
) -> Result<angelscript_core::TypeHash> {
    // Get the type entry
    let type_entry =
        compiler
            .ctx()
            .get_type(type_hash)
            .ok_or_else(|| CompilationError::UnknownType {
                name: format!("{:?}", type_hash),
                span,
            })?;

    // Get the class entry (only classes can have list init)
    let class = type_entry
        .as_class()
        .ok_or_else(|| CompilationError::TypeMismatch {
            message: format!(
                "type '{}' does not support init list syntax (not a class)",
                type_entry.qualified_name()
            ),
            span,
        })?;

    // Check for list_factory or list_construct behavior
    class.behaviors.list_init_func().ok_or_else(|| {
        CompilationError::TypeMismatch {
            message: format!(
                "type '{}' does not support init list syntax (no list factory or list construct behavior)",
                type_entry.qualified_name()
            ),
            span,
        }
    })
}

/// Get the element type expected by the list factory.
///
/// For template types like `array<int>`, the element type is the first type argument.
/// For non-template types with list init, the element type comes from the list factory signature.
fn get_element_type(
    compiler: &ExprCompiler<'_, '_, '_>,
    type_hash: angelscript_core::TypeHash,
    span: Span,
) -> Result<DataType> {
    // Get the type entry
    let type_entry =
        compiler
            .ctx()
            .get_type(type_hash)
            .ok_or_else(|| CompilationError::UnknownType {
                name: format!("{:?}", type_hash),
                span,
            })?;

    // Get the class entry
    let class = type_entry
        .as_class()
        .ok_or_else(|| CompilationError::TypeMismatch {
            message: format!("type '{}' is not a class", type_entry.qualified_name()),
            span,
        })?;

    // For template instances (e.g., array<int>), the element type is the first type argument
    if !class.type_args.is_empty() {
        return Ok(class.type_args[0]);
    }

    // For non-template types, we would need to inspect the list factory signature
    // This is a more complex case that requires looking at the function parameters
    Err(CompilationError::TypeMismatch {
        message: format!(
            "cannot determine element type for '{}' (non-template type with list init not yet supported)",
            type_entry.qualified_name()
        ),
        span,
    })
}

/// Compile a single element of an init list.
fn compile_element(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    element: &InitElement<'_>,
    expected_type: &DataType,
) -> Result<ExprInfo> {
    match element {
        InitElement::Expr(expr) => {
            // Type-check the expression against expected element type
            compiler.check(expr, expected_type)
        }
        InitElement::InitList(nested) => {
            // Nested init list - recursively compile with element type as expected
            compile_init_list(compiler, nested, Some(expected_type))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{ClassEntry, TypeBehaviors, TypeHash, TypeKind, primitives};
    use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_compiler<'a, 'ctx, 'pool>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter<'pool>,
    ) -> ExprCompiler<'a, 'ctx, 'pool> {
        ExprCompiler::new(ctx, emitter, None)
    }

    #[test]
    fn init_list_without_expected_type_fails() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let init_list_expr = InitListExpr {
            ty: None,
            elements: &[],
            span: Span::new(1, 1, 2),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, None);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::TypeMismatch { .. }
        ));
    }

    #[test]
    fn init_list_without_list_behavior_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a class WITHOUT list factory/construct behavior
        let class_hash = TypeHash::from_name("NoListClass");
        let class = ClassEntry::ffi("NoListClass", TypeKind::reference());
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let init_list_expr = InitListExpr {
            ty: None,
            elements: &[],
            span: Span::new(1, 1, 2),
        };

        let expected_type = DataType::simple(class_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(message.contains("list factory") || message.contains("list construct"));
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }

    #[test]
    fn init_list_with_template_type_succeeds() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a template array type with list factory behavior
        let array_int_hash = TypeHash::from_name("array<int>");
        let list_factory_hash = TypeHash::from_name("array<int>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.set_list_factory(list_factory_hash);

        let mut array_class = ClassEntry::ffi("array<int>", TypeKind::reference());
        array_class.behaviors = behaviors;
        // Set template type argument to int
        array_class.type_args = vec![DataType::simple(primitives::INT32)];
        registry.register_type(array_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create init list with int elements: {1, 2, 3}
        let elem1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 2, 1),
        }));
        let elem2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::new(1, 5, 1),
        }));
        let elem3 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(3),
            span: Span::new(1, 8, 1),
        }));

        let elements = arena.alloc_slice_copy(&[
            InitElement::Expr(elem1),
            InitElement::Expr(elem2),
            InitElement::Expr(elem3),
        ]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 9),
        };

        let expected_type = DataType::simple(array_int_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(result.is_ok(), "Init list should compile: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, array_int_hash);
    }

    #[test]
    fn init_list_empty_succeeds() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a template array type with list factory behavior
        let array_int_hash = TypeHash::from_name("array<int>");
        let list_factory_hash = TypeHash::from_name("array<int>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.set_list_factory(list_factory_hash);

        let mut array_class = ClassEntry::ffi("array<int>", TypeKind::reference());
        array_class.behaviors = behaviors;
        array_class.type_args = vec![DataType::simple(primitives::INT32)];
        registry.register_type(array_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Empty init list: {}
        let init_list_expr = InitListExpr {
            ty: None,
            elements: &[],
            span: Span::new(1, 1, 2),
        };

        let expected_type = DataType::simple(array_int_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(
            result.is_ok(),
            "Empty init list should compile: {:?}",
            result
        );
    }
}
