//! User-defined conversions.
//!
//! This module handles conversions via user-defined methods:
//! - `opImplConv` - Implicit conversion method
//! - `opCast` - Explicit cast method
//! - Converting constructors - Single-argument constructors

use angelscript_core::TypeHash;

use crate::context::CompilationContext;

use super::{Conversion, ConversionKind};

/// Find user-defined conversions (constructor, opImplConv, opCast).
pub fn find_user_conversion(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<Conversion> {
    // Try implicit conversion method on source (opImplConv)
    if let Some(method) = find_implicit_conv_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ImplicitConvMethod { method },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // Try constructor conversion on target
    if let Some(ctor) = find_converting_constructor(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ConstructorConversion { constructor: ctor },
            cost: Conversion::COST_USER_IMPLICIT,
            is_implicit: true,
        });
    }

    // Try explicit cast method (opCast)
    if let Some(method) = find_cast_method(source, target, ctx) {
        return Some(Conversion {
            kind: ConversionKind::ExplicitCastMethod { method },
            cost: Conversion::COST_EXPLICIT_ONLY,
            is_implicit: false,
        });
    }

    None
}

/// Find an implicit conversion method (opImplConv) on the source type.
fn find_implicit_conv_method(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source)?.as_class()?;

    for &method_hash in &class.methods {
        if let Some(func) = ctx.get_function(method_hash) {
            let def = &func.def;
            // opImplConv with return type matching target
            if def.name == "opImplConv" && def.return_type.type_hash == target {
                return Some(method_hash);
            }
        }
    }

    None
}

/// Find a converting constructor on the target type.
///
/// A converting constructor is a single-argument constructor that takes
/// the source type.
fn find_converting_constructor(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let target_class = ctx.get_type(target)?.as_class()?;

    // Look for single-argument constructor taking source type
    for &ctor_hash in &target_class.behaviors.constructors {
        if let Some(func) = ctx.get_function(ctor_hash) {
            let def = &func.def;
            if def.params.len() == 1 && def.params[0].data_type.type_hash == source {
                return Some(ctor_hash);
            }
        }
    }

    None
}

/// Find an explicit cast method (opCast) on the source type.
fn find_cast_method(
    source: TypeHash,
    target: TypeHash,
    ctx: &CompilationContext<'_>,
) -> Option<TypeHash> {
    let class = ctx.get_type(source)?.as_class()?;

    for &method_hash in &class.methods {
        if let Some(func) = ctx.get_function(method_hash) {
            let def = &func.def;
            // opCast with return type matching target
            if def.name == "opCast" && def.return_type.type_hash == target {
                return Some(method_hash);
            }
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
        let impl_conv_hash = TypeHash::from_method(source_hash, "opImplConv", &[], true, false);

        // Create Source class with opImplConv method
        let mut source_class = ClassEntry::ffi("Source", TypeKind::reference());
        source_class.methods.push(impl_conv_hash);
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
        let cast_hash = TypeHash::from_method(source_hash, "opCast", &[], true, false);

        // Create Source class with opCast method
        let mut source_class = ClassEntry::ffi("Source", TypeKind::reference());
        source_class.methods.push(cast_hash);
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

        let conv = find_user_conversion(source_hash, target_hash, &ctx);

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

        let conv = find_user_conversion(source_hash, target_hash, &ctx);

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

        let conv = find_user_conversion(source_hash, target_hash, &ctx);

        assert!(conv.is_some());
        let conv = conv.unwrap();
        assert!(!conv.is_implicit); // opCast is explicit only
        assert_eq!(conv.cost, Conversion::COST_EXPLICIT_ONLY);
        assert!(matches!(
            conv.kind,
            ConversionKind::ExplicitCastMethod { method } if method == cast_hash
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

        let conv = find_user_conversion(source_hash, target_hash, &ctx);
        assert!(conv.is_none());
    }

    #[test]
    fn no_conversion_for_primitives() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        // Primitives don't have user-defined conversions
        let conv = find_user_conversion(primitives::INT32, primitives::FLOAT, &ctx);
        assert!(conv.is_none());
    }
}
