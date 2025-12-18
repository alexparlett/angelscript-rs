//! User-defined conversions.
//!
//! This module handles conversions via user-defined methods:
//! - `opImplConv` - Implicit value conversion method
//! - `opConv` - Explicit value conversion method
//! - `opImplCast` - Implicit reference cast method
//! - `opCast` - Explicit reference cast method
//! - Converting constructors - Single-argument constructors
//!
//! ## AngelScript Conversion Semantics
//!
//! | Syntax | Methods Used | Purpose |
//! |--------|--------------|---------|
//! | `type(expr)` | constructor, `opConv`, `opImplConv` | Value conversion |
//! | `cast<type>(expr)` | `opCast`, `opImplCast` | Reference cast (same object, different handle) |
//! | Implicit | non-explicit constructor, `opImplConv`, `opImplCast` | Automatic conversions |

use angelscript_core::{DataType, OperatorBehavior, TypeHash, primitives};

use crate::context::CompilationContext;

use super::{Conversion, ConversionKind};

/// Find user-defined conversions (constructor, opImplConv, opCast).
///
/// Takes full `DataType` references to enable const-correctness checks.
/// Non-const conversion methods cannot be called on const source objects.
pub fn find_user_conversion(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    find_user_conversion_impl(source, target, ctx, false)
}

/// Find user-defined conversion for boolean condition context.
///
/// When compiling boolean expressions in conditions, the compiler will NOT use
/// `bool opImplConv` on reference types, even if the class method is implemented.
/// This is because it is ambiguous whether it is the handle that is verified or
/// the actual object.
///
/// Per AngelScript documentation:
/// > When compiling the boolean expressions in conditions the compiler will not use
/// > the `bool opImplConv` on reference types even if the class method is implemented.
/// > This is because it is ambiguous if it is the handle that is verified or the actual object.
pub fn find_user_conversion_for_condition(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    find_user_conversion_impl(source, target, ctx, true)
}

/// Internal implementation for user conversion lookup.
///
/// Priority order (per AngelScript semantics):
/// 1. opImplConv (implicit value conversion)
/// 2. opImplCast (implicit reference cast)
/// 3. Converting constructor (implicit)
/// 4. opConv (explicit value conversion)
/// 5. opCast (explicit reference cast)
fn find_user_conversion_impl(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
    is_boolean_condition: bool,
) -> Option<Conversion> {
    // 1. Try implicit conversion method on source (opImplConv) - value conversion
    if let Some(method) = find_implicit_conv_method(source, target, ctx, is_boolean_condition) {
        return Some(Conversion {
            kind: ConversionKind::ImplicitConvMethod { method },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // 2. Try implicit cast method on source (opImplCast) - reference cast
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
    if let Some(method) = find_explicit_conv_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ExplicitConvMethod { method },
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    // 5. Try explicit cast method (opCast) - reference cast
    if let Some(method) = find_explicit_cast_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ExplicitRefCastMethod { method },
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    None
}

/// Find an implicit conversion method (opImplConv) on the source type.
///
/// Takes `&DataType` for both source and target for API consistency.
/// Non-const opImplConv methods cannot be called on const objects.
///
/// When `is_boolean_condition` is true, `bool opImplConv` on reference types
/// will be skipped. This is because AngelScript does not use `bool opImplConv`
/// on reference types in boolean conditions to avoid ambiguity between checking
/// the handle vs the object.
fn find_implicit_conv_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
    is_boolean_condition: bool,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // Check if this is a reference type (for the bool opImplConv restriction)
    let is_reference_type = class.type_kind.is_reference();

    // Per AngelScript docs: When compiling boolean expressions in conditions,
    // the compiler will NOT use `bool opImplConv` on reference types.
    // This is because it is ambiguous if it is the handle that is verified
    // or the actual object.
    if is_boolean_condition && target.type_hash == primitives::BOOL && is_reference_type {
        return None;
    }

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpImplConv(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    // Find a method that passes const-correctness check
    for &method_hash in methods {
        if let Some(func) = ctx.get_function(method_hash) {
            // Const-correctness check: non-const methods cannot be called on const objects
            if source.is_effectively_const() && !func.def.is_const() {
                continue;
            }
            return Some(method_hash);
        }
    }

    None
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

    // Find a method that passes const-correctness check
    for &method_hash in methods {
        if let Some(func) = ctx.get_function(method_hash) {
            // Const-correctness check: non-const methods cannot be called on const objects
            if source.is_effectively_const() && !func.def.is_const() {
                continue;
            }
            return Some(method_hash);
        }
    }

    None
}

/// Find an implicit reference cast method (opImplCast) on the source type.
///
/// This is for implicit reference casts - returning a different handle type
/// pointing to the same or related object.
/// Non-const opImplCast methods cannot be called on const objects.
fn find_implicit_cast_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpImplCast(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    // Find a method that passes const-correctness check
    for &method_hash in methods {
        if let Some(func) = ctx.get_function(method_hash) {
            // Const-correctness check: non-const methods cannot be called on const objects
            if source.is_effectively_const() && !func.def.is_const() {
                continue;
            }
            return Some(method_hash);
        }
    }

    None
}

/// Find an explicit reference cast method (opCast) on the source type.
///
/// This is for explicit reference casts via `cast<type>(expr)` syntax.
/// Non-const opCast methods cannot be called on const objects.
fn find_explicit_cast_method(
    source: &DataType,
    target: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source.type_hash)?.as_class()?;

    // O(1) lookup using OperatorBehavior as key
    let op = OperatorBehavior::OpCast(target.type_hash);
    let methods = class.behaviors.get_operator(op)?;

    // Find a method that passes const-correctness check
    for &method_hash in methods {
        if let Some(func) = ctx.get_function(method_hash) {
            // Const-correctness check: non-const methods cannot be called on const objects
            if source.is_effectively_const() && !func.def.is_const() {
                continue;
            }
            return Some(method_hash);
        }
    }

    None
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
        let conv = find_user_conversion(&source, &target, &ctx);

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
        let conv = find_user_conversion(&source, &target, &ctx);

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
    fn find_cast_method_is_explicit() {
        let (registry, source_hash, target_hash, cast_hash) = setup_context_with_cast_method();
        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(source_hash);
        let target = DataType::simple(target_hash);
        let conv = find_user_conversion(&source, &target, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(!conv.is_implicit); // opCast is explicit only
        assert_eq!(conv.cost, Conversion::COST_EXPLICIT_ONLY);
        assert!(matches!(
            conv.kind,
            ConversionKind::ExplicitRefCastMethod { method } if method == cast_hash
        ));
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
        let conv = find_user_conversion(&source, &target, &ctx);
        assert!(conv.is_none());
    }

    #[test]
    fn no_conversion_for_primitives() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        // Primitives don't have user-defined conversions
        let source = DataType::simple(primitives::INT32);
        let target = DataType::simple(primitives::FLOAT);
        let conv = find_user_conversion(&source, &target, &ctx);
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
        let conv = find_user_conversion(&source, &target, &ctx);

        // Explicit constructor should NOT be found for implicit conversion
        assert!(conv.is_none());
    }

    // =========================================================================
    // Tests for bool opImplConv restriction on reference types
    // =========================================================================

    fn setup_class_with_bool_opimplconv(
        registry: &mut SymbolRegistry,
        class_name: &str,
        is_reference_type: bool,
    ) -> (TypeHash, TypeHash) {
        let class_hash = TypeHash::from_name(class_name);
        let method_hash = TypeHash::from_method(class_hash, "opImplConv", &[]);

        // Create class with opImplConv returning bool
        let type_kind = if is_reference_type {
            TypeKind::reference()
        } else {
            TypeKind::value::<i32>()
        };
        let mut class = ClassEntry::ffi(class_name, type_kind);
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

        (class_hash, method_hash)
    }

    #[test]
    fn bool_opimplconv_on_value_type_found_normally() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let (class_hash, method_hash) =
            setup_class_with_bool_opimplconv(&mut registry, "ValueClass", false);

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(class_hash);
        let target = DataType::simple(primitives::BOOL);

        // Normal conversion should find the method
        let conv = find_user_conversion(&source, &target, &ctx);
        assert!(conv.is_some());
        assert!(matches!(
            conv.unwrap().kind,
            ConversionKind::ImplicitConvMethod { method } if method == method_hash
        ));
    }

    #[test]
    fn bool_opimplconv_on_value_type_found_in_condition() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let (class_hash, method_hash) =
            setup_class_with_bool_opimplconv(&mut registry, "ValueClass", false);

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(class_hash);
        let target = DataType::simple(primitives::BOOL);

        // Condition conversion should ALSO find the method for VALUE types
        let conv = find_user_conversion_for_condition(&source, &target, &ctx);
        assert!(conv.is_some());
        assert!(matches!(
            conv.unwrap().kind,
            ConversionKind::ImplicitConvMethod { method } if method == method_hash
        ));
    }

    #[test]
    fn bool_opimplconv_on_reference_type_found_normally() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let (class_hash, method_hash) =
            setup_class_with_bool_opimplconv(&mut registry, "RefClass", true);

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(class_hash);
        let target = DataType::simple(primitives::BOOL);

        // Normal conversion should find the method
        let conv = find_user_conversion(&source, &target, &ctx);
        assert!(conv.is_some());
        assert!(matches!(
            conv.unwrap().kind,
            ConversionKind::ImplicitConvMethod { method } if method == method_hash
        ));
    }

    #[test]
    fn bool_opimplconv_on_reference_type_not_found_in_condition() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let (class_hash, _) = setup_class_with_bool_opimplconv(&mut registry, "RefClass", true);

        let ctx = CompilationContext::new(&registry);

        let source = DataType::simple(class_hash);
        let target = DataType::simple(primitives::BOOL);

        // Condition conversion should NOT find the method for REFERENCE types
        // Per AngelScript docs: bool opImplConv on reference types is ambiguous
        // (is it the handle or the object being checked?)
        let conv = find_user_conversion_for_condition(&source, &target, &ctx);
        assert!(
            conv.is_none(),
            "bool opImplConv on reference type should NOT be used in boolean conditions"
        );
    }

    #[test]
    fn non_bool_opimplconv_on_reference_type_found_in_condition() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let class_hash = TypeHash::from_name("RefClass");
        let method_hash = TypeHash::from_method(class_hash, "opImplConv", &[]);

        // Create reference type class with opImplConv returning int (not bool)
        let mut class = ClassEntry::ffi("RefClass", TypeKind::reference());
        class
            .behaviors
            .add_operator(OperatorBehavior::OpImplConv(primitives::INT32), method_hash);
        registry.register_type(class.into()).unwrap();

        // Register opImplConv method returning int (NOT bool)
        let mut traits = FunctionTraits::default();
        traits.is_const = true;
        let method_def = FunctionDef::new(
            method_hash,
            "opImplConv".to_string(),
            vec![],
            vec![],
            DataType::simple(primitives::INT32), // Returns int, not bool
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
        let target = DataType::simple(primitives::INT32);

        // Non-bool opImplConv on reference type SHOULD be found even in condition context
        // (the restriction only applies to bool opImplConv)
        let conv = find_user_conversion_for_condition(&source, &target, &ctx);
        assert!(conv.is_some());
        assert!(matches!(
            conv.unwrap().kind,
            ConversionKind::ImplicitConvMethod { method } if method == method_hash
        ));
    }
}
