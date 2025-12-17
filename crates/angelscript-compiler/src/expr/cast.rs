//! Cast expression compilation.
//!
//! Handles `cast<Type>(expr)` syntax for reference casts in AngelScript.
//!
//! ## Reference Casts vs Value Conversions
//!
//! `cast<>` is specifically for reference casts (same object, different handle type):
//! - `cast<Derived@>(base@)` - Downcast in class hierarchy
//! - `cast<Interface@>(obj@)` - Cast via `opCast`/`opImplCast` methods
//!
//! For value conversions, use constructor syntax: `Type(expr)`

use angelscript_core::{CompilationError, TypeHash};
use angelscript_parser::ast::CastExpr;

use super::{ExprCompiler, Result, emit_conversion};
use crate::conversion::{Conversion, ConversionKind, find_conversion};
use crate::expr_info::ExprInfo;
use crate::type_resolver::TypeResolver;

/// Compile a cast expression: `cast<Type>(expr)`
///
/// Reference casts allow changing the handle type while referring to the same object.
/// This includes:
/// - Class hierarchy casts (base to derived, derived to base)
/// - Interface casts via `opCast`/`opImplCast` methods
pub fn compile_cast<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &CastExpr<'ast>,
) -> Result<ExprInfo> {
    let span = expr.span;

    // 1. Resolve the target type
    let target_type = {
        let mut resolver = TypeResolver::new(compiler.ctx_mut());
        resolver.resolve(&expr.target_type)?
    };

    // 2. Compile the source expression
    let source_info = compiler.infer(expr.expr)?;
    let source_type = &source_info.data_type;

    // 3. Identity cast - no conversion needed
    if source_type.type_hash == target_type.type_hash {
        // Just need to handle const/handle modifier changes
        return Ok(ExprInfo::rvalue(target_type));
    }

    // 4. Try to find a conversion (including explicit-only conversions)
    if let Some(conv) = find_conversion(source_type, &target_type, compiler.ctx()) {
        // For cast<>, we allow explicit conversions
        // Check if this is a valid cast conversion
        if is_valid_cast_conversion(&conv) {
            emit_conversion(compiler.emitter(), &conv);
            return Ok(ExprInfo::rvalue(target_type));
        }
    }

    // 5. Handle hierarchy casts (derived to base, base to derived)
    if source_type.is_handle
        && target_type.is_handle
        && let Some(conv) =
            find_hierarchy_cast(source_type.type_hash, target_type.type_hash, compiler.ctx())
    {
        emit_conversion(compiler.emitter(), &conv);
        return Ok(ExprInfo::rvalue(target_type));
    }

    // 6. No valid cast found
    Err(CompilationError::InvalidCast {
        from: compiler
            .ctx()
            .get_type(source_type.type_hash)
            .map(|t| t.qualified_name().to_string())
            .unwrap_or_else(|| format!("{:?}", source_type.type_hash)),
        to: compiler
            .ctx()
            .get_type(target_type.type_hash)
            .map(|t| t.qualified_name().to_string())
            .unwrap_or_else(|| format!("{:?}", target_type.type_hash)),
        span,
    })
}

/// Check if a conversion is valid for cast<> syntax.
///
/// Cast allows:
/// - Reference casts (opCast, opImplCast, hierarchy casts)
/// - NOT value conversions (those use Type(expr) syntax)
fn is_valid_cast_conversion(conv: &Conversion) -> bool {
    matches!(
        conv.kind,
        ConversionKind::Identity
            | ConversionKind::HandleToConst
            | ConversionKind::DerivedToBase { .. }
            | ConversionKind::ClassToInterface { .. }
            | ConversionKind::ReferenceCast { .. }
            | ConversionKind::ImplicitCastMethod { .. }
            | ConversionKind::ExplicitRefCastMethod { .. }
    )
}

/// Find a hierarchy cast between handle types.
///
/// This handles downcasting (base to derived) which requires runtime type checking.
fn find_hierarchy_cast(
    source_hash: TypeHash,
    target_hash: TypeHash,
    ctx: &crate::context::CompilationContext<'_>,
) -> Option<Conversion> {
    // Check if target is derived from source (downcast)
    if is_derived_from(target_hash, source_hash, ctx) {
        // Downcast requires runtime type check via Cast opcode
        return Some(Conversion {
            kind: ConversionKind::ReferenceCast {
                target: target_hash,
            },
            cost: Conversion::COST_REFERENCE_CAST,
            is_implicit: false, // Downcasts are explicit only
        });
    }

    // Check if source is derived from target (upcast) - should be found by find_conversion
    // but handle it here for completeness
    if is_derived_from(source_hash, target_hash, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ReferenceCast {
                target: target_hash,
            },
            cost: Conversion::COST_REFERENCE_CAST,
            is_implicit: true,
        });
    }

    // Check interface implementation
    if let Some(source_class) = ctx.get_type(source_hash).and_then(|t| t.as_class())
        && source_class.interfaces.contains(&target_hash)
    {
        return Some(Conversion {
            kind: ConversionKind::ReferenceCast {
                target: target_hash,
            },
            cost: Conversion::COST_REFERENCE_CAST,
            is_implicit: true,
        });
    }

    None
}

/// Check if source is derived from target (walks inheritance chain).
fn is_derived_from(
    source: TypeHash,
    target: TypeHash,
    ctx: &crate::context::CompilationContext<'_>,
) -> bool {
    let mut current = source;
    while let Some(class) = ctx.get_type(current).and_then(|t| t.as_class()) {
        if let Some(base) = class.base_class {
            if base == target {
                return true;
            }
            current = base;
        } else {
            break;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{ClassEntry, DataType, Span, TypeKind};
    use angelscript_parser::ast::{Expr, Ident, IdentExpr, TypeBase, TypeExpr, TypeSuffix};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_compiler<'a, 'ctx, 'pool>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter<'pool>,
    ) -> ExprCompiler<'a, 'ctx, 'pool> {
        ExprCompiler::new(ctx, emitter, None)
    }

    fn make_ident_expr<'a>(arena: &'a Bump, name: &'a str, span: Span) -> &'a Expr<'a> {
        arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new(name, span),
            type_args: &[],
            span,
        }))
    }

    #[test]
    fn cast_derived_to_base_handle() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create base class
        let base_hash = TypeHash::from_name("Entity");
        let base_class = ClassEntry::ffi("Entity", TypeKind::reference());
        registry.register_type(base_class.into()).unwrap();

        // Create derived class
        let derived_hash = TypeHash::from_name("Player");
        let derived_class = ClassEntry::ffi("Player", TypeKind::reference()).with_base(base_hash);
        registry.register_type(derived_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a variable of type Player@
        ctx.declare_local(
            "player".to_string(),
            DataType::simple(derived_hash).as_handle(),
            false,
            Span::default(),
        )
        .unwrap();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create cast<Entity@>(player) expression
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let target_type = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Entity", Span::new(1, 6, 6))),
            &[],
            suffixes,
            Span::new(1, 6, 7),
        );

        let player_ident = make_ident_expr(&arena, "player", Span::new(1, 15, 6));

        let cast_expr = CastExpr {
            target_type,
            expr: player_ident,
            span: Span::new(1, 1, 22),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_cast(&mut compiler, &cast_expr);

        assert!(result.is_ok(), "Cast should succeed: {:?}", result);
        let info = result.unwrap();

        // Result should be Entity@
        assert_eq!(info.data_type.type_hash, base_hash);
        assert!(info.data_type.is_handle);
    }

    #[test]
    fn cast_base_to_derived_handle() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create base class
        let base_hash = TypeHash::from_name("Entity");
        let base_class = ClassEntry::ffi("Entity", TypeKind::reference());
        registry.register_type(base_class.into()).unwrap();

        // Create derived class
        let derived_hash = TypeHash::from_name("Player");
        let derived_class = ClassEntry::ffi("Player", TypeKind::reference()).with_base(base_hash);
        registry.register_type(derived_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a variable of type Entity@
        ctx.declare_local(
            "entity".to_string(),
            DataType::simple(base_hash).as_handle(),
            false,
            Span::default(),
        )
        .unwrap();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create cast<Player@>(entity) expression (downcast)
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let target_type = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Player", Span::new(1, 6, 6))),
            &[],
            suffixes,
            Span::new(1, 6, 7),
        );

        let entity_ident = make_ident_expr(&arena, "entity", Span::new(1, 15, 6));

        let cast_expr = CastExpr {
            target_type,
            expr: entity_ident,
            span: Span::new(1, 1, 22),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_cast(&mut compiler, &cast_expr);

        assert!(result.is_ok(), "Downcast should succeed: {:?}", result);
        let info = result.unwrap();

        // Result should be Player@
        assert_eq!(info.data_type.type_hash, derived_hash);
        assert!(info.data_type.is_handle);
    }

    #[test]
    fn cast_unrelated_types_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create two unrelated classes
        let player_hash = TypeHash::from_name("Player");
        let _enemy_hash = TypeHash::from_name("Enemy");

        let player_class = ClassEntry::ffi("Player", TypeKind::reference());
        let enemy_class = ClassEntry::ffi("Enemy", TypeKind::reference());
        registry.register_type(player_class.into()).unwrap();
        registry.register_type(enemy_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a variable of type Player@
        ctx.declare_local(
            "player".to_string(),
            DataType::simple(player_hash).as_handle(),
            false,
            Span::default(),
        )
        .unwrap();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create cast<Enemy@>(player) expression - should fail
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let target_type = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Enemy", Span::new(1, 6, 5))),
            &[],
            suffixes,
            Span::new(1, 6, 6),
        );

        let player_ident = make_ident_expr(&arena, "player", Span::new(1, 14, 6));

        let cast_expr = CastExpr {
            target_type,
            expr: player_ident,
            span: Span::new(1, 1, 21),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_cast(&mut compiler, &cast_expr);

        assert!(result.is_err(), "Cast between unrelated types should fail");
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::InvalidCast { .. }
        ));
    }
}
