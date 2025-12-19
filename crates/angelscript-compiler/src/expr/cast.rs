//! Cast expression compilation.
//!
//! Handles `cast<Type>(expr)` syntax for reference casts in AngelScript.
//!
//! ## Reference Casts vs Value Conversions
//!
//! `cast<>` is specifically for reference casts (same object, different handle type):
//! - `cast<Derived@>(base@)` - Downcast in class hierarchy
//! - `cast<Base@>(derived@)` - Upcast in class hierarchy
//! - `cast<Interface@>(obj@)` - Cast to implemented interface
//! - `cast<OtherType@>(obj@)` - Cast via `opCast`/`opImplCast` methods
//!
//! The `opCast` and `opImplCast` operators enable casting between unrelated types
//! by returning a handle to a different type (often a member variable).
//!
//! For value conversions, use constructor syntax: `Type(expr)`

use angelscript_core::{CompilationError, TypeHash};
use angelscript_parser::ast::CastExpr;

use super::{ExprCompiler, Result};
use crate::conversion::find_cast;
use crate::expr_info::ExprInfo;
use crate::type_resolver::TypeResolver;

/// Compile a cast expression: `cast<Type>(expr)`
///
/// Reference casts allow changing the handle type while referring to the same object.
/// This includes:
/// - Class hierarchy casts (base to derived, derived to base)
/// - Interface casts (class to implemented interface)
/// - User-defined casts via `opCast`/`opImplCast` methods
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

    // 3. Identity cast - no cast needed
    if source_type.type_hash == target_type.type_hash {
        // Just need to handle const/handle modifier changes
        return Ok(ExprInfo::rvalue(target_type));
    }

    // 4. Handle casts only work between handle types
    if source_type.is_handle && target_type.is_handle {
        // 4a. Try hierarchy casts (derived to base, base to derived, interface)
        if let Some(()) = try_hierarchy_cast(source_type.type_hash, target_type.type_hash, compiler)
        {
            return Ok(ExprInfo::rvalue(target_type));
        }

        // 4b. Try user-defined cast operators (opCast, opImplCast)
        if let Some((method_hash, _is_implicit)) =
            find_cast(source_type, &target_type, compiler.ctx())
        {
            // Emit method call for the cast operator (0 args since it's a getter-style method)
            compiler.emitter().emit_call_method(method_hash, 0);
            return Ok(ExprInfo::rvalue(target_type));
        }
    }

    // 5. No valid cast found
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

/// Try to emit a hierarchy cast between handle types.
///
/// This handles:
/// - Upcasts (derived to base) - always valid
/// - Downcasts (base to derived) - requires runtime type check
/// - Interface casts (class to implemented interface)
///
/// Returns `Some(())` if a valid cast was found and bytecode emitted.
fn try_hierarchy_cast(
    source_hash: TypeHash,
    target_hash: TypeHash,
    compiler: &mut ExprCompiler<'_, '_, '_>,
) -> Option<()> {
    let ctx = compiler.ctx();

    // Check if source is derived from target (upcast) - always valid
    if ctx.is_type_derived_from(source_hash, target_hash) {
        // Upcast: just reinterpret the handle, no runtime check needed
        compiler.emitter().emit_cast(target_hash);
        return Some(());
    }

    // Check if target is derived from source (downcast) - requires runtime check
    if ctx.is_type_derived_from(target_hash, source_hash) {
        // Downcast: emit Cast opcode which does runtime type check
        compiler.emitter().emit_cast(target_hash);
        return Some(());
    }

    // Check interface implementation
    if let Some(source_class) = ctx.get_type(source_hash).and_then(|t| t.as_class())
        && source_class.interfaces.contains(&target_hash)
    {
        // Interface cast: reinterpret as interface handle
        compiler.emitter().emit_cast(target_hash);
        return Some(());
    }

    None
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
    fn cast_unrelated_types_without_opcast_fails() {
        let mut registry = SymbolRegistry::with_primitives();

        // Create two unrelated classes without opCast
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

        // Create cast<Enemy@>(player) expression - should fail (no opCast defined)
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

        assert!(
            result.is_err(),
            "Cast between unrelated types without opCast should fail"
        );
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::InvalidCast { .. }
        ));
    }

    #[test]
    fn cast_via_opcast_method() {
        use angelscript_core::{
            FunctionDef, FunctionEntry, FunctionTraits, OperatorBehavior, Visibility,
        };

        let mut registry = SymbolRegistry::with_primitives();

        // Create two unrelated classes: MyObjA and MyObjB
        let obj_a_hash = TypeHash::from_name("MyObjA");
        let obj_b_hash = TypeHash::from_name("MyObjB");

        // Register MyObjB first (target of cast)
        let obj_b_class = ClassEntry::ffi("MyObjB", TypeKind::reference());
        registry.register_type(obj_b_class.into()).unwrap();

        // Register MyObjA with opCast returning MyObjB@
        let opcast_hash = TypeHash::from_method(obj_a_hash, "opCast", &[]);
        let mut obj_a_class = ClassEntry::ffi("MyObjA", TypeKind::reference());
        // The target of opCast is a handle to MyObjB
        let target_handle_hash = obj_b_hash; // For simplicity, behaviors key on base type
        obj_a_class
            .behaviors
            .add_operator(OperatorBehavior::OpCast(target_handle_hash), opcast_hash);
        registry.register_type(obj_a_class.into()).unwrap();

        // Register the opCast method itself
        let opcast_def = FunctionDef::new(
            opcast_hash,
            "opCast".to_string(),
            vec![],
            vec![],
            DataType::simple(obj_b_hash).as_handle(), // Returns MyObjB@
            Some(obj_a_hash),
            FunctionTraits::default(),
            true, // const method
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(opcast_def))
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a variable of type MyObjA@
        ctx.declare_local(
            "objA".to_string(),
            DataType::simple(obj_a_hash).as_handle(),
            false,
            Span::default(),
        )
        .unwrap();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create cast<MyObjB@>(objA) expression - should succeed via opCast
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let target_type = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("MyObjB", Span::new(1, 6, 6))),
            &[],
            suffixes,
            Span::new(1, 6, 7),
        );

        let obj_a_ident = make_ident_expr(&arena, "objA", Span::new(1, 15, 4));

        let cast_expr = CastExpr {
            target_type,
            expr: obj_a_ident,
            span: Span::new(1, 1, 20),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_cast(&mut compiler, &cast_expr);

        assert!(
            result.is_ok(),
            "Cast via opCast should succeed: {:?}",
            result
        );
        let info = result.unwrap();

        // Result should be MyObjB@
        assert_eq!(info.data_type.type_hash, obj_b_hash);
        assert!(info.data_type.is_handle);
    }

    #[test]
    fn cast_via_opimplcast_method() {
        use angelscript_core::{
            FunctionDef, FunctionEntry, FunctionTraits, OperatorBehavior, Visibility,
        };

        let mut registry = SymbolRegistry::with_primitives();

        // Create two unrelated classes: MyObjA and MyObjC
        let obj_a_hash = TypeHash::from_name("MyObjA");
        let obj_c_hash = TypeHash::from_name("MyObjC");

        // Register MyObjC first (target of implicit cast)
        let obj_c_class = ClassEntry::ffi("MyObjC", TypeKind::reference());
        registry.register_type(obj_c_class.into()).unwrap();

        // Register MyObjA with opImplCast returning MyObjC@
        let opimplcast_hash = TypeHash::from_method(obj_a_hash, "opImplCast", &[]);
        let mut obj_a_class = ClassEntry::ffi("MyObjA", TypeKind::reference());
        obj_a_class
            .behaviors
            .add_operator(OperatorBehavior::OpImplCast(obj_c_hash), opimplcast_hash);
        registry.register_type(obj_a_class.into()).unwrap();

        // Register the opImplCast method itself
        let opimplcast_def = FunctionDef::new(
            opimplcast_hash,
            "opImplCast".to_string(),
            vec![],
            vec![],
            DataType::simple(obj_c_hash).as_handle(), // Returns MyObjC@
            Some(obj_a_hash),
            FunctionTraits::default(),
            true, // const method
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(opimplcast_def))
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a variable of type MyObjA@
        ctx.declare_local(
            "objA".to_string(),
            DataType::simple(obj_a_hash).as_handle(),
            false,
            Span::default(),
        )
        .unwrap();

        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();

        // Create cast<MyObjC@>(objA) expression - should succeed via opImplCast
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let target_type = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("MyObjC", Span::new(1, 6, 6))),
            &[],
            suffixes,
            Span::new(1, 6, 7),
        );

        let obj_a_ident = make_ident_expr(&arena, "objA", Span::new(1, 15, 4));

        let cast_expr = CastExpr {
            target_type,
            expr: obj_a_ident,
            span: Span::new(1, 1, 20),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_cast(&mut compiler, &cast_expr);

        assert!(
            result.is_ok(),
            "Cast via opImplCast should succeed: {:?}",
            result
        );
        let info = result.unwrap();

        // Result should be MyObjC@
        assert_eq!(info.data_type.type_hash, obj_c_hash);
        assert!(info.data_type.is_handle);
    }
}
