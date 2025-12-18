//! Init list expression compilation.
//!
//! Handles `{elem1, elem2, ...}` syntax for initializer lists.
//!
//! ## Usage
//!
//! Init lists require:
//! 1. An expected type (from context, like `array<int> arr = {...}`)
//! 2. The target type must have a list factory or list construct behavior with a pattern
//!
//! If either condition is not met, compilation fails with an error.
//!
//! ## List Patterns
//!
//! The element types are determined by the stored `ListPattern`:
//!
//! - `Repeat(T)`: Each element must be of type T. If T has a list factory, nested init lists
//!   delegate to T's pattern (e.g., `array<array<int>>` delegates `{1,2}` to `array<int>`).
//!
//! - `RepeatTuple([T1, T2, ...])`: Each element must be a nested init list `{t1, t2, ...}` that
//!   matches the tuple types. Inner braces are structural grouping, NOT nested delegation.
//!   Example: `dictionary<K, V>` with `RepeatTuple([K, V])` expects `{{k1, v1}, {k2, v2}}`.
//!
//! - `Fixed([T1, T2, ...])`: Exactly N elements with types matching the fixed pattern.
//!   Example: `MyVec3` with `Fixed([float, float, float])` expects `{1.0, 2.0, 3.0}`.

use angelscript_core::list_buffer::ListPattern;
use angelscript_core::{CompilationError, DataType, ListBehavior, Span};
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

    // 2. Look up the type's list behavior (function + pattern)
    let list_behavior = get_list_behavior(compiler, target_type.type_hash, span)?;

    // 3. Compile elements based on the pattern
    match &list_behavior.pattern {
        ListPattern::Repeat(elem_type_hash) => {
            // Each element should be of the element type
            let elem_type = DataType::simple(*elem_type_hash);
            for element in expr.elements.iter() {
                compile_element(compiler, element, &elem_type, span)?;
            }
        }
        ListPattern::RepeatTuple(tuple_types) => {
            // Each element must be a nested init list with tuple_types.len() elements
            // Inner braces are structural tuples, NOT nested delegation
            for element in expr.elements.iter() {
                compile_tuple_element(compiler, element, tuple_types, span)?;
            }
        }
        ListPattern::Fixed(types) => {
            // Exact number of elements required
            if expr.elements.len() != types.len() {
                return Err(CompilationError::TypeMismatch {
                    message: format!(
                        "init list expects exactly {} elements, found {}",
                        types.len(),
                        expr.elements.len()
                    ),
                    span,
                });
            }
            for (element, type_hash) in expr.elements.iter().zip(types) {
                let expected_type = DataType::simple(*type_hash);
                compile_element(compiler, element, &expected_type, span)?;
            }
        }
    }

    // 4. Emit the list factory/construct call
    let element_count = expr.elements.len() as u8;
    compiler
        .emitter()
        .emit_call(list_behavior.func_hash, element_count);

    Ok(ExprInfo::rvalue(target_type))
}

/// Get the list behavior (function + pattern) for a type.
///
/// Returns the first list behavior, preferring factories over constructs.
fn get_list_behavior(
    compiler: &ExprCompiler<'_, '_, '_>,
    type_hash: angelscript_core::TypeHash,
    span: Span,
) -> Result<ListBehavior> {
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

    // Get the first list behavior (factories preferred over constructs)
    class
        .behaviors
        .list_behaviors()
        .first()
        .cloned()
        .ok_or_else(|| CompilationError::TypeMismatch {
            message: format!(
                "type '{}' does not support init list syntax (no list factory or list construct behavior)",
                type_entry.qualified_name()
            ),
            span,
        })
}

/// Compile a single element of an init list.
///
/// For nested init lists, this function checks if the expected type has a list factory.
/// If so, it delegates to that type's pattern (nested delegation).
/// If not, it's an error - you can't use `{...}` for a type without list support.
fn compile_element(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    element: &InitElement<'_>,
    expected_type: &DataType,
    _outer_span: Span,
) -> Result<ExprInfo> {
    match element {
        InitElement::Expr(expr) => {
            // Type-check the expression against expected element type
            compiler.check(expr, expected_type)
        }
        InitElement::InitList(nested) => {
            // Nested init list: check if expected type has a list factory
            // If yes: delegate to nested type's list factory (e.g., array<array<int>>)
            // If no: error - the type doesn't support init list syntax
            let expected_hash = expected_type.type_hash;

            // Try to get the nested type's list behavior
            if get_list_behavior(compiler, expected_hash, nested.span).is_ok() {
                // Delegate to nested type's list factory
                // This recursively compiles the inner init list with the nested type's pattern
                compile_init_list(compiler, nested, Some(expected_type))
            } else {
                // No list factory on expected type - error
                let type_name = compiler
                    .ctx()
                    .get_type(expected_hash)
                    .map(|t| t.qualified_name().to_string())
                    .unwrap_or_else(|| format!("{:?}", expected_hash));
                Err(CompilationError::TypeMismatch {
                    message: format!(
                        "type '{}' does not support init list syntax; \
                         nested {{...}} can only be used if the element type has a list factory",
                        type_name
                    ),
                    span: nested.span,
                })
            }
        }
    }
}

/// Compile a tuple element for RepeatTuple patterns.
///
/// The element MUST be a nested init list (inner braces) with exactly tuple_types.len() elements.
/// Inner braces are structural grouping, NOT nested delegation to another list factory.
///
/// Example: dictionary<string, int> has pattern RepeatTuple([string, int])
/// Input: {{"key", 1}, {"key2", 2}}
/// Each inner {"key", 1} is a structural tuple that gets flattened to [key, value, key, value, ...]
fn compile_tuple_element(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    element: &InitElement<'_>,
    tuple_types: &[angelscript_core::TypeHash],
    outer_span: Span,
) -> Result<()> {
    match element {
        InitElement::InitList(nested) => {
            // Validate that the nested list has exactly tuple_types.len() elements
            if nested.elements.len() != tuple_types.len() {
                return Err(CompilationError::TypeMismatch {
                    message: format!(
                        "tuple init list element must have exactly {} elements, found {}",
                        tuple_types.len(),
                        nested.elements.len()
                    ),
                    span: nested.span,
                });
            }

            // Compile each element with its corresponding type
            // Note: These elements CAN themselves be nested init lists if their expected
            // type has a list factory (nested delegation within tuple)
            for (elem, type_hash) in nested.elements.iter().zip(tuple_types) {
                let expected_type = DataType::simple(*type_hash);
                compile_element(compiler, elem, &expected_type, nested.span)?;
            }

            Ok(())
        }
        InitElement::Expr(_) => {
            // For tuple patterns, direct expressions are not allowed - must be {elem1, elem2, ...}
            Err(CompilationError::TypeMismatch {
                message: format!(
                    "RepeatTuple init list elements must be {{...}} tuples with {} elements",
                    tuple_types.len()
                ),
                span: outer_span,
            })
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
    fn init_list_with_repeat_pattern_succeeds() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a template array type with list factory behavior and Repeat pattern
        let array_int_hash = TypeHash::from_name("array<int>");
        let list_factory_hash = TypeHash::from_name("array<int>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_factory(ListBehavior::new(
            list_factory_hash,
            ListPattern::Repeat(primitives::INT32),
        ));

        let mut array_class = ClassEntry::ffi("array<int>", TypeKind::reference());
        array_class.behaviors = behaviors;
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
        behaviors.add_list_factory(ListBehavior::new(
            list_factory_hash,
            ListPattern::Repeat(primitives::INT32),
        ));

        let mut array_class = ClassEntry::ffi("array<int>", TypeKind::reference());
        array_class.behaviors = behaviors;
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

    #[test]
    fn init_list_repeat_tuple_pattern_succeeds() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a PairList type with RepeatTuple pattern (int, float pairs)
        // Using primitives to avoid NoStringFactory error
        let pair_list_hash = TypeHash::from_name("PairList<int,float>");
        let list_factory_hash = TypeHash::from_name("PairList<int,float>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_factory(ListBehavior::new(
            list_factory_hash,
            ListPattern::RepeatTuple(vec![primitives::INT32, primitives::FLOAT]),
        ));

        let mut pair_list_class = ClassEntry::ffi("PairList<int,float>", TypeKind::reference());
        pair_list_class.behaviors = behaviors;
        registry.register_type(pair_list_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create: {{1, 1.0}, {2, 2.0}}
        let key1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 3, 1),
        }));
        let value1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(1.0),
            span: Span::new(1, 6, 3),
        }));
        let pair1 = InitListExpr {
            ty: None,
            elements: arena.alloc_slice_copy(&[InitElement::Expr(key1), InitElement::Expr(value1)]),
            span: Span::new(1, 2, 8),
        };

        let key2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::new(1, 13, 1),
        }));
        let value2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(2.0),
            span: Span::new(1, 16, 3),
        }));
        let pair2 = InitListExpr {
            ty: None,
            elements: arena.alloc_slice_copy(&[InitElement::Expr(key2), InitElement::Expr(value2)]),
            span: Span::new(1, 12, 8),
        };

        let elements =
            arena.alloc_slice_copy(&[InitElement::InitList(pair1), InitElement::InitList(pair2)]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 22),
        };

        let expected_type = DataType::simple(pair_list_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(
            result.is_ok(),
            "RepeatTuple init list should compile: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, pair_list_hash);
    }

    #[test]
    fn init_list_repeat_tuple_wrong_element_count_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a PairList type with RepeatTuple pattern (int, float pairs)
        let pair_list_hash = TypeHash::from_name("PairList<int,float>");
        let list_factory_hash = TypeHash::from_name("PairList<int,float>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_factory(ListBehavior::new(
            list_factory_hash,
            ListPattern::RepeatTuple(vec![primitives::INT32, primitives::FLOAT]),
        ));

        let mut pair_list_class = ClassEntry::ffi("PairList<int,float>", TypeKind::reference());
        pair_list_class.behaviors = behaviors;
        registry.register_type(pair_list_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create: {{1, 1.0, 99}} - 3 elements in tuple (should fail, expects 2)
        let elem1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 3, 1),
        }));
        let elem2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(1.0),
            span: Span::new(1, 6, 3),
        }));
        let extra = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(99),
            span: Span::new(1, 11, 2),
        }));
        let pair1 = InitListExpr {
            ty: None,
            elements: arena.alloc_slice_copy(&[
                InitElement::Expr(elem1),
                InitElement::Expr(elem2),
                InitElement::Expr(extra),
            ]),
            span: Span::new(1, 2, 12),
        };

        let elements = arena.alloc_slice_copy(&[InitElement::InitList(pair1)]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 14),
        };

        let expected_type = DataType::simple(pair_list_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(
                    message.contains("exactly 2 elements"),
                    "Expected error about element count, got: {}",
                    message
                );
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }

    #[test]
    fn init_list_repeat_tuple_direct_expr_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a PairList type with RepeatTuple pattern (int, float pairs)
        let pair_list_hash = TypeHash::from_name("PairList<int,float>");
        let list_factory_hash = TypeHash::from_name("PairList<int,float>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_factory(ListBehavior::new(
            list_factory_hash,
            ListPattern::RepeatTuple(vec![primitives::INT32, primitives::FLOAT]),
        ));

        let mut pair_list_class = ClassEntry::ffi("PairList<int,float>", TypeKind::reference());
        pair_list_class.behaviors = behaviors;
        registry.register_type(pair_list_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create: {42} - direct expression instead of {int, float} tuple (should fail)
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::new(1, 2, 2),
        }));

        let elements = arena.alloc_slice_copy(&[InitElement::Expr(expr)]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 4),
        };

        let expected_type = DataType::simple(pair_list_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(
                    message.contains("RepeatTuple") || message.contains("tuples"),
                    "Expected error about tuple requirement, got: {}",
                    message
                );
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }

    #[test]
    fn init_list_fixed_pattern_succeeds() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a Vec3 type with Fixed pattern
        let vec3_hash = TypeHash::from_name("Vec3");
        let list_construct_hash = TypeHash::from_name("Vec3::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_construct(ListBehavior::new(
            list_construct_hash,
            ListPattern::Fixed(vec![
                primitives::FLOAT,
                primitives::FLOAT,
                primitives::FLOAT,
            ]),
        ));

        let mut vec3_class = ClassEntry::ffi("Vec3", TypeKind::value_sized(12, 4, true));
        vec3_class.behaviors = behaviors;
        registry.register_type(vec3_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create init list: {1.0, 2.0, 3.0}
        let x = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(1.0),
            span: Span::new(1, 2, 3),
        }));
        let y = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(2.0),
            span: Span::new(1, 7, 3),
        }));
        let z = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(3.0),
            span: Span::new(1, 12, 3),
        }));

        let elements = arena.alloc_slice_copy(&[
            InitElement::Expr(x),
            InitElement::Expr(y),
            InitElement::Expr(z),
        ]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 15),
        };

        let expected_type = DataType::simple(vec3_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(
            result.is_ok(),
            "Fixed init list should compile: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, vec3_hash);
    }

    #[test]
    fn init_list_fixed_pattern_wrong_count_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create a Vec3 type with Fixed pattern expecting 3 floats
        let vec3_hash = TypeHash::from_name("Vec3");
        let list_construct_hash = TypeHash::from_name("Vec3::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_construct(ListBehavior::new(
            list_construct_hash,
            ListPattern::Fixed(vec![
                primitives::FLOAT,
                primitives::FLOAT,
                primitives::FLOAT,
            ]),
        ));

        let mut vec3_class = ClassEntry::ffi("Vec3", TypeKind::value_sized(12, 4, true));
        vec3_class.behaviors = behaviors;
        registry.register_type(vec3_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create init list with only 2 elements: {1.0, 2.0}
        let x = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(1.0),
            span: Span::new(1, 2, 3),
        }));
        let y = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(2.0),
            span: Span::new(1, 7, 3),
        }));

        let elements = arena.alloc_slice_copy(&[InitElement::Expr(x), InitElement::Expr(y)]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 10),
        };

        let expected_type = DataType::simple(vec3_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(
                    message.contains("exactly 3 elements"),
                    "Expected error about element count, got: {}",
                    message
                );
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }

    #[test]
    fn init_list_nested_delegation() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create inner array<int> type
        let array_int_hash = TypeHash::from_name("array<int>");
        let array_int_factory_hash = TypeHash::from_name("array<int>::$list");

        let mut array_int_behaviors = TypeBehaviors::new();
        array_int_behaviors.add_list_factory(ListBehavior::new(
            array_int_factory_hash,
            ListPattern::Repeat(primitives::INT32),
        ));

        let mut array_int_class = ClassEntry::ffi("array<int>", TypeKind::reference());
        array_int_class.behaviors = array_int_behaviors;
        registry.register_type(array_int_class.into()).unwrap();

        // Create outer array<array<int>> type
        let array_array_int_hash = TypeHash::from_name("array<array<int>>");
        let array_array_int_factory_hash = TypeHash::from_name("array<array<int>>::$list");

        let mut array_array_int_behaviors = TypeBehaviors::new();
        array_array_int_behaviors.add_list_factory(ListBehavior::new(
            array_array_int_factory_hash,
            ListPattern::Repeat(array_int_hash), // Element type is array<int>
        ));

        let mut array_array_int_class = ClassEntry::ffi("array<array<int>>", TypeKind::reference());
        array_array_int_class.behaviors = array_array_int_behaviors;
        registry
            .register_type(array_array_int_class.into())
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create nested init list: {{1, 2}, {3, 4, 5}}
        let elem1_1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 3, 1),
        }));
        let elem1_2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::new(1, 6, 1),
        }));
        let inner1 = InitListExpr {
            ty: None,
            elements: arena
                .alloc_slice_copy(&[InitElement::Expr(elem1_1), InitElement::Expr(elem1_2)]),
            span: Span::new(1, 2, 6),
        };

        let elem2_1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(3),
            span: Span::new(1, 11, 1),
        }));
        let elem2_2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(4),
            span: Span::new(1, 14, 1),
        }));
        let elem2_3 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(5),
            span: Span::new(1, 17, 1),
        }));
        let inner2 = InitListExpr {
            ty: None,
            elements: arena.alloc_slice_copy(&[
                InitElement::Expr(elem2_1),
                InitElement::Expr(elem2_2),
                InitElement::Expr(elem2_3),
            ]),
            span: Span::new(1, 10, 9),
        };

        let elements =
            arena.alloc_slice_copy(&[InitElement::InitList(inner1), InitElement::InitList(inner2)]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 20),
        };

        let expected_type = DataType::simple(array_array_int_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(
            result.is_ok(),
            "Nested array init list should compile via delegation: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, array_array_int_hash);
    }

    #[test]
    fn init_list_nested_without_list_factory_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create array<int> type with Repeat pattern
        let array_int_hash = TypeHash::from_name("array<int>");
        let array_int_factory_hash = TypeHash::from_name("array<int>::$list");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_factory(ListBehavior::new(
            array_int_factory_hash,
            ListPattern::Repeat(primitives::INT32), // Element type is int (no list factory)
        ));

        let mut array_int_class = ClassEntry::ffi("array<int>", TypeKind::reference());
        array_int_class.behaviors = behaviors;
        registry.register_type(array_int_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Try to use nested init list for int elements: {{1, 2}}
        // This should FAIL because int doesn't have a list factory
        let elem1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 3, 1),
        }));
        let elem2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::new(1, 6, 1),
        }));
        let inner = InitListExpr {
            ty: None,
            elements: arena.alloc_slice_copy(&[InitElement::Expr(elem1), InitElement::Expr(elem2)]),
            span: Span::new(1, 2, 6),
        };

        let elements = arena.alloc_slice_copy(&[InitElement::InitList(inner)]);

        let init_list_expr = InitListExpr {
            ty: None,
            elements,
            span: Span::new(1, 1, 8),
        };

        let expected_type = DataType::simple(array_int_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_init_list(&mut compiler, &init_list_expr, Some(&expected_type));

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(
                    message.contains("does not support init list syntax"),
                    "Expected error about type not supporting init list, got: {}",
                    message
                );
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }
}
