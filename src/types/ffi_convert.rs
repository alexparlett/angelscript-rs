//! AST to FFI type conversion utilities.
//!
//! This module provides functions to convert arena-allocated AST types
//! to owned FFI types for storage in `Arc<FfiRegistry>`.
//!
//! # Conversion Functions
//!
//! - `type_expr_to_ffi` - Convert `TypeExpr<'ast>` to `FfiDataType`
//! - `param_type_to_ffi` - Convert `ParamType<'ast>` to `FfiDataType`
//! - `function_param_to_ffi` - Convert `FunctionParam<'ast>` to `FfiParam`
//! - `return_type_to_ffi` - Convert `ReturnType<'ast>` to `FfiDataType`
//! - `signature_to_ffi_function` - Convert `FunctionSignatureDecl<'ast>` to `FfiFunctionDef`

use crate::ast::types::{ParamType, PrimitiveType as AstPrimitiveType, TypeBase, TypeExpr, TypeSuffix};
use crate::ast::{FunctionParam, FunctionSignatureDecl, RefKind, ReturnType};
use crate::ffi::NativeFn;
use crate::semantic::types::type_def::{
    BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE, INT8_TYPE,
    UINT16_TYPE, UINT32_TYPE, UINT64_TYPE, UINT8_TYPE, VOID_TYPE,
};
use crate::semantic::types::{DataType, RefModifier};
use crate::types::{FfiDataType, FfiExpr, FfiFunctionDef, FfiParam};

/// Convert an AST TypeExpr to FfiDataType.
///
/// Primitive types are resolved immediately. User-defined types become
/// unresolved and will be resolved during the install phase.
///
/// Note: This function does not handle reference modifiers (they are on ParamType).
/// Use `param_type_to_ffi` for parameters with reference modifiers.
pub fn type_expr_to_ffi(type_expr: &TypeExpr<'_>) -> FfiDataType {
    type_expr_to_ffi_with_ref(type_expr, RefModifier::None)
}

/// Convert an AST TypeExpr to FfiDataType with a specific reference modifier.
fn type_expr_to_ffi_with_ref(type_expr: &TypeExpr<'_>, ref_modifier: RefModifier) -> FfiDataType {
    // Start with the is_const from the leading const qualifier
    let is_const = type_expr.is_const;
    let mut is_handle = false;
    let mut is_handle_to_const = false;

    // Process suffixes for handle and trailing const
    for suffix in type_expr.suffixes.iter() {
        match suffix {
            TypeSuffix::Handle { is_const: handle_const } => {
                is_handle = true;
                is_handle_to_const = *handle_const;
            }
        }
    }

    // Build qualified name from scope + base
    let build_name = |base_name: &str| -> String {
        if let Some(scope) = &type_expr.scope {
            let mut parts: Vec<&str> = scope.segments.iter().map(|s| s.name).collect();
            parts.push(base_name);
            parts.join("::")
        } else {
            base_name.to_string()
        }
    };

    // Resolve the base type
    match &type_expr.base {
        TypeBase::Primitive(prim) => {
            let type_id = primitive_to_type_id(prim);
            FfiDataType::Resolved(DataType {
                type_id,
                is_const,
                is_handle,
                is_handle_to_const,
                ref_modifier,
            })
        }
        TypeBase::Named(ident) => {
            let name = build_name(ident.name);

            // Check if it has template arguments
            if !type_expr.template_args.is_empty() {
                let args: Vec<FfiDataType> = type_expr
                    .template_args
                    .iter()
                    .map(|arg| type_expr_to_ffi(arg))
                    .collect();
                FfiDataType::unresolved_template(
                    name,
                    args,
                    is_const,
                    is_handle,
                    is_handle_to_const,
                    ref_modifier,
                )
            } else {
                FfiDataType::unresolved(name, is_const, is_handle, is_handle_to_const, ref_modifier)
            }
        }
        TypeBase::TemplateParam(ident) => {
            // Template parameter like "T" in array<class T>
            let name = build_name(ident.name);
            FfiDataType::unresolved(name, is_const, is_handle, is_handle_to_const, ref_modifier)
        }
        TypeBase::Auto | TypeBase::Unknown => {
            // Auto/unknown types shouldn't appear in FFI declarations
            FfiDataType::unresolved("auto", is_const, is_handle, is_handle_to_const, ref_modifier)
        }
    }
}

/// Convert a primitive AST type to its TypeId.
fn primitive_to_type_id(prim: &AstPrimitiveType) -> crate::semantic::types::TypeId {
    match prim {
        AstPrimitiveType::Void => VOID_TYPE,
        AstPrimitiveType::Bool => BOOL_TYPE,
        AstPrimitiveType::Int8 => INT8_TYPE,
        AstPrimitiveType::Int16 => INT16_TYPE,
        AstPrimitiveType::Int => INT32_TYPE,
        AstPrimitiveType::Int64 => INT64_TYPE,
        AstPrimitiveType::UInt8 => UINT8_TYPE,
        AstPrimitiveType::UInt16 => UINT16_TYPE,
        AstPrimitiveType::UInt => UINT32_TYPE,
        AstPrimitiveType::UInt64 => UINT64_TYPE,
        AstPrimitiveType::Float => FLOAT_TYPE,
        AstPrimitiveType::Double => DOUBLE_TYPE,
    }
}

/// Convert RefKind to RefModifier.
fn ref_kind_to_modifier(ref_kind: RefKind) -> RefModifier {
    match ref_kind {
        RefKind::None => RefModifier::None,
        RefKind::Ref => RefModifier::InOut, // Plain & is inout by default
        RefKind::RefIn => RefModifier::In,
        RefKind::RefOut => RefModifier::Out,
        RefKind::RefInOut => RefModifier::InOut,
    }
}

/// Convert an AST ParamType to FfiDataType.
///
/// This handles the reference modifier from the parameter type.
pub fn param_type_to_ffi(param_type: &ParamType<'_>) -> FfiDataType {
    let ref_modifier = ref_kind_to_modifier(param_type.ref_kind);
    type_expr_to_ffi_with_ref(&param_type.ty, ref_modifier)
}

/// Convert an AST FunctionParam to FfiParam.
pub fn function_param_to_ffi(param: &FunctionParam<'_>) -> FfiParam {
    let data_type = param_type_to_ffi(&param.ty);
    let default_value = param.default.and_then(|expr| FfiExpr::from_ast(expr));
    let name = param.name.map(|n| n.name.to_string()).unwrap_or_default();

    match default_value {
        Some(expr) => FfiParam::with_default(name, data_type, expr),
        None => FfiParam::new(name, data_type),
    }
}

/// Convert an AST ReturnType to FfiDataType.
///
/// ReturnType is a struct containing a TypeExpr and an is_ref flag.
/// If is_ref is true, we apply the InOut reference modifier.
pub fn return_type_to_ffi(return_type: &ReturnType<'_>) -> FfiDataType {
    // Check if the return type is void
    if matches!(return_type.ty.base, TypeBase::Primitive(AstPrimitiveType::Void)) {
        return FfiDataType::resolved(DataType::simple(VOID_TYPE));
    }

    // Convert the type expression
    let ref_modifier = if return_type.is_ref {
        RefModifier::InOut  // Return by reference
    } else {
        RefModifier::None
    };

    type_expr_to_ffi_with_ref(&return_type.ty, ref_modifier)
}

/// Convert a FunctionSignatureDecl to FfiFunctionDef.
///
/// This is used for both standalone functions and methods.
pub fn signature_to_ffi_function(
    sig: &FunctionSignatureDecl<'_>,
    native_fn: NativeFn,
) -> FfiFunctionDef {
    use crate::semantic::types::type_def::FunctionId;

    let name = sig.name.name.to_string();
    let params: Vec<FfiParam> = sig.params.iter().map(function_param_to_ffi).collect();
    let return_type = return_type_to_ffi(&sig.return_type);

    FfiFunctionDef::new(FunctionId::next_ffi(), name)
        .with_params(params)
        .with_return_type(return_type)
        .with_native_fn(native_fn)
        .with_const(sig.is_const)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Parser;
    use crate::ffi::CallContext;
    use bumpalo::Bump;

    fn dummy_native_fn() -> NativeFn {
        NativeFn::new(|_ctx: &mut CallContext| Ok(()))
    }

    #[test]
    fn primitive_type_conversion() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("int", &arena).unwrap();
        let ffi_type = type_expr_to_ffi(&type_expr);

        assert!(ffi_type.is_resolved());
        if let FfiDataType::Resolved(dt) = ffi_type {
            assert_eq!(dt.type_id, INT32_TYPE);
            assert!(!dt.is_const);
            assert!(!dt.is_handle);
        }
    }

    #[test]
    fn const_primitive_type() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("const int", &arena).unwrap();
        let ffi_type = type_expr_to_ffi(&type_expr);

        assert!(ffi_type.is_resolved());
        if let FfiDataType::Resolved(dt) = ffi_type {
            assert_eq!(dt.type_id, INT32_TYPE);
            assert!(dt.is_const);
        }
    }

    #[test]
    fn user_type_unresolved() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("MyClass", &arena).unwrap();
        let ffi_type = type_expr_to_ffi(&type_expr);

        assert!(ffi_type.is_unresolved());
    }

    #[test]
    fn handle_type() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("MyClass@", &arena).unwrap();
        let ffi_type = type_expr_to_ffi(&type_expr);

        assert!(ffi_type.is_unresolved());
        if let FfiDataType::Unresolved { is_handle, .. } = ffi_type {
            assert!(is_handle);
        }
    }

    #[test]
    fn template_type() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("array<int>", &arena).unwrap();
        let ffi_type = type_expr_to_ffi(&type_expr);

        assert!(ffi_type.is_unresolved());
        if let FfiDataType::Unresolved { base, .. } = ffi_type {
            if let crate::types::UnresolvedBaseType::Template { name, args } = base {
                assert_eq!(name, "array");
                assert_eq!(args.len(), 1);
                assert!(args[0].is_resolved()); // int is resolved
            } else {
                panic!("Expected template base type");
            }
        }
    }

    #[test]
    fn function_param_conversion() {
        let arena = Bump::new();

        let sig = Parser::function_decl("void foo(int x, float y = 1.0)", &arena).unwrap();

        let param1 = function_param_to_ffi(&sig.params[0]);
        assert_eq!(param1.name, "x");
        assert!(param1.data_type.is_resolved());
        assert!(param1.default_value.is_none());

        let param2 = function_param_to_ffi(&sig.params[1]);
        assert_eq!(param2.name, "y");
        assert!(param2.default_value.is_some());
    }

    #[test]
    fn function_param_with_ref() {
        let arena = Bump::new();

        let sig = Parser::function_decl("void foo(int &in x, int &out y, int &inout z)", &arena).unwrap();

        let param1 = function_param_to_ffi(&sig.params[0]);
        if let FfiDataType::Resolved(dt) = &param1.data_type {
            assert_eq!(dt.ref_modifier, RefModifier::In);
        }

        let param2 = function_param_to_ffi(&sig.params[1]);
        if let FfiDataType::Resolved(dt) = &param2.data_type {
            assert_eq!(dt.ref_modifier, RefModifier::Out);
        }

        let param3 = function_param_to_ffi(&sig.params[2]);
        if let FfiDataType::Resolved(dt) = &param3.data_type {
            assert_eq!(dt.ref_modifier, RefModifier::InOut);
        }
    }

    #[test]
    fn signature_conversion() {
        let arena = Bump::new();

        let sig = Parser::function_decl("int getValue() const", &arena).unwrap();
        let ffi_func = signature_to_ffi_function(&sig, dummy_native_fn());

        assert_eq!(ffi_func.name, "getValue");
        assert!(ffi_func.params.is_empty());
        assert!(ffi_func.is_const());
        assert!(ffi_func.return_type.is_resolved());
    }

    #[test]
    fn void_return_type() {
        let arena = Bump::new();

        let sig = Parser::function_decl("void doSomething()", &arena).unwrap();
        let ffi_func = signature_to_ffi_function(&sig, dummy_native_fn());

        assert!(ffi_func.return_type.is_resolved());
        if let FfiDataType::Resolved(dt) = &ffi_func.return_type {
            assert_eq!(dt.type_id, VOID_TYPE);
        }
    }

    #[test]
    fn scoped_type_name() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("Game::Entity", &arena).unwrap();
        let ffi_type = type_expr_to_ffi(&type_expr);

        assert!(ffi_type.is_unresolved());
        if let FfiDataType::Unresolved { base, .. } = ffi_type {
            if let crate::types::UnresolvedBaseType::Simple(name) = base {
                assert_eq!(name, "Game::Entity");
            } else {
                panic!("Expected simple base type");
            }
        }
    }

    #[test]
    fn ref_kind_to_modifier_conversion() {
        assert_eq!(ref_kind_to_modifier(RefKind::None), RefModifier::None);
        assert_eq!(ref_kind_to_modifier(RefKind::RefIn), RefModifier::In);
        assert_eq!(ref_kind_to_modifier(RefKind::RefOut), RefModifier::Out);
        assert_eq!(ref_kind_to_modifier(RefKind::RefInOut), RefModifier::InOut);
        assert_eq!(ref_kind_to_modifier(RefKind::Ref), RefModifier::InOut);
    }
}
