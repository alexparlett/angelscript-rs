//! Operator-based conversions.
//!
//! This module handles conversions via conversion operators:
//! - `opImplConv` - Implicit value conversion operator
//! - `opConv` - Explicit value conversion operator
//! - `opImplCast` - Implicit reference cast operator
//! - `opCast` - Explicit reference cast operator
//! - Converting constructors - Single-argument constructors
//!
//! ## AngelScript Conversion Semantics
//!
//! | Syntax | Operators Used | Purpose |
//! |--------|----------------|---------|
//! | `type(expr)` | constructor, `opConv`, `opImplConv` | Value conversion |
//! | `cast<type>(expr)` | `opCast`, `opImplCast` | Reference cast (same object, different handle) |
//! | Implicit | non-explicit constructor, `opImplConv`, `opImplCast` | Automatic conversions |

use angelscript_core::{DataType, OperatorBehavior, TypeHash};

use crate::context::CompilationContext;

use super::{Conversion, ConversionKind};

/// Find the first method that passes const-correctness check.
///
/// Non-const methods cannot be called on const objects.
fn find_const_correct_method(
    methods: &[TypeHash],
    source_is_const: bool,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    for &method_hash in methods {
        if let Some(func) = ctx.get_function(method_hash) {
            if source_is_const && !func.def.is_const() {
                continue;
            }
            return Some(method_hash);
        }
    }
    None
}

/// Find operator-based conversions (constructor, opImplConv, opConv).
///
/// Takes full `DataType` references to enable const-correctness checks.
/// Non-const conversion operators cannot be called on const source objects.
///
/// # Parameters
/// - `implicit_only`: If true, only return implicit conversions (opImplConv, constructors).
///   If false, also include explicit-only conversions (opConv for `Type(expr)` syntax).
///
/// NOTE: opCast is NOT included here. It's for reference casts via `cast<Type>(expr)`.
/// Use `find_cast_operator` for cast<> compilation.
pub fn find_operator_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
    implicit_only: bool,
) -> Option<Conversion> {
    // 1. Try implicit conversion method on source (opImplConv) - value conversion
    if let Some(method) = find_implicit_conv_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ImplicitConvMethod { method },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // 2. Try implicit cast method on source (opImplCast) - reference cast
    // This returns a handle and is valid for implicit handle assignments like:
    //   OtherType@ ref = myObj;  // Calls myObj.opImplCast()
    if let Some(method) = find_implicit_cast_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ImplicitCastMethod { method },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // 3. Try constructor conversion on target
    if let Some(ctor) = find_converting_constructor(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ConstructorConversion { constructor: ctor },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // 4. Try explicit conversion method (opConv) - value conversion
    // This is for Type(expr) syntax, not cast<Type>(expr)
    // Only include if caller allows explicit conversions
    if !implicit_only && let Some(method) = find_explicit_conv_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ExplicitConvMethod { method },
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    // NOTE: opCast is intentionally NOT checked here.
    // opCast is explicit-only and can ONLY be invoked via cast<Type>(expr) syntax.
    // Use find_cast_operator() for cast<> expression compilation.

    None
}

/// Find an implicit conversion method (opImplConv) on the source type.
///
/// Takes `&DataType` for both source and target for API consistency.
/// Non-const opImplConv methods cannot be called on const objects.
fn find_implicit_conv_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpImplConv(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    find_const_correct_method(methods, source.is_effectively_const(), ctx)
}

/// Find a converting constructor on the target type.
///
/// A converting constructor is a single-argument constructor that takes
/// the source type. Const-correctness is checked: cannot pass a const source
/// to a parameter that doesn't accept const.
///
/// Explicit constructors (marked with `explicit` keyword) are NOT considered
/// for implicit conversions - they can only be called via direct `TypeName(args)` syntax.
fn find_converting_constructor(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let target_class = ctx.get_type(target.type_hash)?.as_class()?;

    // Look for single-argument constructor taking source type
    for &ctor_hash in &target_class.behaviors.constructors {
        if let Some(func) = ctx.get_function(ctor_hash) {
            let def = &func.def;

            // Skip explicit constructors - they can't be used for implicit conversions
            if def.traits.is_explicit {
                continue;
            }

            if def.params.len() == 1 && def.params[0].data_type.type_hash == source.type_hash {
                let param_type = &def.params[0].data_type;

                // Const-correctness: if source is const, parameter must accept const
                // Just check the const keyword on the parameter type
                if source.is_effectively_const() && !param_type.is_effectively_const() {
                    continue; // Skip - can't pass const to non-const param
                }

                return Some(ctor_hash);
            }
        }
    }

    None
}

/// Find an explicit conversion method (opConv) on the source type.
///
/// This is for explicit value conversions, used by `type(expr)` syntax
/// when no constructor is found.
/// Non-const opConv methods cannot be called on const objects.
fn find_explicit_conv_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpConv(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    find_const_correct_method(methods, source.is_effectively_const(), ctx)
}

/// Find an implicit reference cast method (opImplCast) on the source type.
///
/// This is for implicit reference casts - returning a different handle type
/// pointing to the same or related object.
/// Non-const opImplCast methods cannot be called on const objects.
pub(super) fn find_implicit_cast_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpImplCast(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    find_const_correct_method(methods, source.is_effectively_const(), ctx)
}

/// Find an explicit reference cast method (opCast) on the source type.
///
/// This is for explicit reference casts via `cast<type>(expr)` syntax.
/// Non-const opCast methods cannot be called on const objects.
pub(super) fn find_explicit_cast_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpCast(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    find_const_correct_method(methods, source.is_effectively_const(), ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        ClassEntry, DataType, FunctionDef, FunctionEntry, FunctionTraits, Param, TypeBehaviors,
        TypeKind, Visibility, primitives,
    };
    use angelscript_registry::SymbolRegistry;

    fn setup_context_with_conversion_class() -> (SymbolRegistry, TypeHash, TypeHash, TypeHash) {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let source_hash = TypeHash::from_name("Source");
        let target_hash = TypeHash::from_name("Target");

        // Create opImplConv method hash
        let impl_conv_hash = TypeHash::from_method(source_hash, "opImplConv", &[]);

        // Create Source class with opImplConv operator behavior
        let mut source_class = ClassEntry::ffi("Source", TypeKind::reference());
        source_class
            .behaviors
            .add_operator(OperatorBehavior::OpImplConv(target_hash), impl_conv_hash);
        registry.register_type(source_class.into()).unwrap();

        // Register opImplConv method
        let impl_conv_def = FunctionDef::new(
            impl_conv_hash,
            "opImplConv".to_string(),
            vec![],
            vec![],
            DataType::simple(target_hash),
            Some(source_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(impl_conv_def))
            .unwrap();

        // Create Target class
        let target_class = ClassEntry::ffi("Target", TypeKind::reference());
        registry.register_type(target_class.into()).unwrap();

        (registry, source_hash, target_hash, impl_conv_hash)
    }

    fn setup_context_with_constructor_conversion() -> (SymbolRegistry, TypeHash, TypeHash, TypeHash)
    {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let source_hash = TypeHash::from_name("Source");
        let target_hash = TypeHash::from_name("Target");
        let ctor_hash = TypeHash::from_constructor(target_hash, &[source_hash]);

        // Create Source class
        let source_class = ClassEntry::ffi("Source", TypeKind::reference());
        registry.register_type(source_class.into()).unwrap();

        // Create Target class with converting constructor
        let mut target_class = ClassEntry::ffi("Target", TypeKind::reference());
        target_class.behaviors = TypeBehaviors {
            constructors: vec![ctor_hash],
            ..Default::default()
        };
        registry.register_type(target_class.into()).unwrap();

        // Register constructor
        let ctor_def = FunctionDef::new(
            ctor_hash,
            "$ctor".to_string(),
            vec![],
            vec![Param {
                name: "value".to_string(),
                data_type: DataType::simple(source_hash),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            Some(target_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(ctor_def))
            .unwrap();

        (registry, source_hash, target_hash, ctor_hash)
    }

    fn setup_context_with_cast_method() -> (SymbolRegistry, TypeHash, TypeHash, TypeHash) {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let source_hash = TypeHash::from_name("Source");
        let target_hash = TypeHash::from_name("Target");
        let cast_hash = TypeHash::from_method(source_hash, "opCast", &[]);

        // Create Source class with opCast operator behavior
        let mut source_class = ClassEntry::ffi("Source", TypeKind::reference());
        source_class
            .behaviors
            .add_operator(OperatorBehavior::OpCast(target_hash), cast_hash);
        registry.register_type(source_class.into()).unwrap();

        // Register opCast method
        let cast_def = FunctionDef::new(
            cast_hash,
            "opCast".to_string(),
            vec![],
            vec![],
            DataType::simple(target_hash),
            Some(source_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(cast_def))
            .unwrap();

        // Create Target class
        let target_class = ClassEntry::ffi("Target", TypeKind::reference());
        registry.register_type(target_class.into()).unwrap();

        (registry, source_hash, target_hash, cast_hash)
    }

    #[test]
    fn find_implicit_conversion_method() {
        let (registry, source_hash, target_hash, impl_conv_hash) =
            setup_context_with_conversion_class();
        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let conv = find_operator_conversion(&source, &target, &ctx, true);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert_eq!(conv.cost, Conversion::COST_USER_IMPLICIT);
        assert!(matches!(
            conv.kind,
            ConversionKind::ImplicitConvMethod { method } if method == impl_conv_hash
        ));
    }

    #[test]
    fn find_converting_constructor() {
        let (registry, source_hash, target_hash, ctor_hash) =
            setup_context_with_constructor_conversion();
        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let conv = find_operator_conversion(&source, &target, &ctx, true);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(conv.is_implicit);
        assert_eq!(conv.cost, Conversion::COST_USER_IMPLICIT);
        assert!(matches!(
            conv.kind,
            ConversionKind::ConstructorConversion { constructor } if constructor == ctor_hash
        ));
    }

    #[test]
    fn opcast_not_found_by_find_operator_conversion() {
        // opCast is explicit-only and should ONLY be usable via cast<>() syntax.
        // It should NOT be found by find_operator_conversion.
        let (registry, source_hash, target_hash, _cast_hash) = setup_context_with_cast_method();
        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let conv = find_operator_conversion(&source, &target, &ctx, false);

        // opCast should NOT be found - it's only for cast<>() syntax
        assert!(
            conv.is_none(),
            "opCast should not be found by find_operator_conversion"
        );
    }

    #[test]
    fn opcast_found_by_find_explicit_cast_method() {
        // opCast should be found by find_explicit_cast_method
        let (registry, source_hash, target_hash, cast_hash) = setup_context_with_cast_method();
        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let result = find_explicit_cast_method(&source, &target, &ctx);

        assert!(
            result.is_some(),
            "opCast should be found by find_explicit_cast_method"
        );
        assert_eq!(result.unwrap(), cast_hash);
    }

    #[test]
    fn no_conversion_between_unrelated_types() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let source_hash = TypeHash::from_name("Source");
        let target_hash = TypeHash::from_name("Target");

        // Create classes without any conversion methods
        let source_class = ClassEntry::ffi("Source", TypeKind::reference());
        let target_class = ClassEntry::ffi("Target", TypeKind::reference());
        registry.register_type(source_class.into()).unwrap();
        registry.register_type(target_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let conv = find_operator_conversion(&source, &target, &ctx, true);
        assert!(conv.is_none());
    }

    #[test]
    fn no_conversion_for_primitives() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        // Primitives don't have user-defined conversions
        let source = DataType::simple(primitives::INT32);
        let target = DataType::simple(primitives::FLOAT);
        let conv = find_operator_conversion(&source, &target, &ctx, true);
        assert!(conv.is_none());
    }

    #[test]
    fn explicit_constructor_not_used_for_implicit_conversion() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let source_hash = TypeHash::from_name("Source");
        let target_hash = TypeHash::from_name("Target");
        let ctor_hash = TypeHash::from_constructor(target_hash, &[source_hash]);

        // Create Source class
        let source_class = ClassEntry::ffi("Source", TypeKind::reference());
        registry.register_type(source_class.into()).unwrap();

        // Create Target class with explicit converting constructor
        let mut target_class = ClassEntry::ffi("Target", TypeKind::reference());
        target_class.behaviors = TypeBehaviors {
            constructors: vec![ctor_hash],
            ..Default::default()
        };
        registry.register_type(target_class.into()).unwrap();

        // Register constructor with is_explicit = true
        let mut traits = FunctionTraits::default();
        traits.is_explicit = true;
        let ctor_def = FunctionDef::new(
            ctor_hash,
            "$ctor".to_string(),
            vec![],
            vec![Param {
                name: "value".to_string(),
                data_type: DataType::simple(source_hash),
                has_default: false,
                if_handle_then_const: false,
            }],
            DataType::void(),
            Some(target_hash),
            traits,
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(ctor_def))
            .unwrap();

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let conv = find_operator_conversion(&source, &target, &ctx, true);

        // Explicit constructor should NOT be found for implicit conversion
        assert!(conv.is_none());
    }

    #[test]
    fn bool_opimplconv_found() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let class_hash = TypeHash::from_name("ValueClass");
        let method_hash = TypeHash::from_method(class_hash, "opImplConv", &[]);

        // Create class with opImplConv returning bool
        let mut class = ClassEntry::ffi("ValueClass", TypeKind::value::<i32>());
        class
            .behaviors
            .add_operator(OperatorBehavior::OpImplConv(primitives::BOOL), method_hash);
        registry.register_type(class.into()).unwrap();

        // Register opImplConv method returning bool
        let mut traits = FunctionTraits::default();
        traits.is_const = true;
        let method_def = FunctionDef::new(
            method_hash,
            "opImplConv".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::BOOL),
            Some(class_hash),
            traits,
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(method_def))
            .unwrap();

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(class_hash);
        let target = DataType::simple(primitives::BOOL);

        // Conversion should find the method
        let conv = find_operator_conversion(&source, &target, &ctx, true);
        assert!(conv.is_some());
        assert!(matches!(
            conv.unwrap().kind,
            ConversionKind::ImplicitConvMethod { method } if method == method_hash
        ));
    }
}
