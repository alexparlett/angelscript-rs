//! AST to FFI type conversion utilities.
//!
//! This module provides functions to convert arena-allocated AST types
//! to owned FFI types for storage in `Arc<FfiRegistry>`.
//!
//! # Conversion Functions
//!
//! - `type_expr_to_data_type` - Convert `TypeExpr<'ast>` to `DataType`
//! - `param_type_to_data_type` - Convert `ParamType<'ast>` to `DataType`
//! - `function_param_to_ffi` - Convert `FunctionParam<'ast>` to `FfiParam`
//! - `return_type_to_data_type` - Convert `ReturnType<'ast>` to `DataType`
//! - `signature_to_ffi_function` - Convert `FunctionSignatureDecl<'ast>` to `FfiFunctionDef`

use crate::ast::types::{ParamType, PrimitiveType as AstPrimitiveType, TypeBase, TypeExpr, TypeSuffix};
use crate::ast::{FunctionParam, FunctionSignatureDecl, RefKind, ReturnType};
use crate::ffi::NativeFn;
use crate::types::{primitive_hashes, TypeHash};
use crate::semantic::types::{DataType, RefModifier};
use crate::types::{FfiExpr, FfiFunctionDef, FfiParam};

/// Convert an AST TypeExpr to DataType.
///
/// All types are resolved immediately using deterministic TypeHash:
/// - Primitives use their well-known hashes
/// - User types use `TypeHash::from_name()`
/// - Template parameters use `TypeHash::SELF`
///
/// Note: This function does not handle reference modifiers (they are on ParamType).
/// Use `param_type_to_data_type` for parameters with reference modifiers.
pub fn type_expr_to_data_type(type_expr: &TypeExpr<'_>) -> DataType {
    type_expr_to_data_type_with_ref(type_expr, RefModifier::None)
}

/// Convert an AST TypeExpr to DataType with a specific reference modifier.
fn type_expr_to_data_type_with_ref(type_expr: &TypeExpr<'_>, ref_modifier: RefModifier) -> DataType {
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
    let type_hash = match &type_expr.base {
        TypeBase::Primitive(prim) => primitive_to_type_hash(prim),
        TypeBase::Named(ident) => {
            let name = build_name(ident.name);
            // Template instantiations (e.g., array<int>) happen at compile time.
            // For FFI method signatures on template types, use SELF.
            if !type_expr.template_args.is_empty() {
                // This is a template instantiation like array<T> in a method signature.
                // Use SELF since the actual instantiation happens at compile time.
                primitive_hashes::SELF
            } else {
                TypeHash::from_name(&name)
            }
        }
        TypeBase::TemplateParam(ident) => {
            // Template parameter like "T" in array<class T>
            // Use TypeHash::from_name() for the param name.
            // This will be remapped to the actual template param hash during install_type.
            TypeHash::from_name(ident.name)
        }
        TypeBase::Auto | TypeBase::Unknown => {
            // Auto/unknown types use VARIABLE_PARAM (?)
            primitive_hashes::VARIABLE_PARAM
        }
    };

    DataType {
        type_hash,
        is_const,
        is_handle,
        is_handle_to_const,
        ref_modifier,
    }
}

/// Convert a primitive AST type to its TypeHash.
fn primitive_to_type_hash(prim: &AstPrimitiveType) -> TypeHash {
    match prim {
        AstPrimitiveType::Void => primitive_hashes::VOID,
        AstPrimitiveType::Bool => primitive_hashes::BOOL,
        AstPrimitiveType::Int8 => primitive_hashes::INT8,
        AstPrimitiveType::Int16 => primitive_hashes::INT16,
        AstPrimitiveType::Int => primitive_hashes::INT32,
        AstPrimitiveType::Int64 => primitive_hashes::INT64,
        AstPrimitiveType::UInt8 => primitive_hashes::UINT8,
        AstPrimitiveType::UInt16 => primitive_hashes::UINT16,
        AstPrimitiveType::UInt => primitive_hashes::UINT32,
        AstPrimitiveType::UInt64 => primitive_hashes::UINT64,
        AstPrimitiveType::Float => primitive_hashes::FLOAT,
        AstPrimitiveType::Double => primitive_hashes::DOUBLE,
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

/// Convert an AST ParamType to DataType.
///
/// This handles the reference modifier from the parameter type.
pub fn param_type_to_data_type(param_type: &ParamType<'_>) -> DataType {
    let ref_modifier = ref_kind_to_modifier(param_type.ref_kind);
    type_expr_to_data_type_with_ref(&param_type.ty, ref_modifier)
}

/// Convert an AST FunctionParam to FfiParam.
pub fn function_param_to_ffi(param: &FunctionParam<'_>) -> FfiParam {
    let data_type = param_type_to_data_type(&param.ty);
    let default_value = param.default.and_then(|expr| FfiExpr::from_ast(expr));
    let name = param.name.map(|n| n.name.to_string()).unwrap_or_default();

    match default_value {
        Some(expr) => FfiParam::with_default(name, data_type, expr),
        None => FfiParam::new(name, data_type),
    }
}

/// Convert an AST ReturnType to DataType.
///
/// ReturnType is a struct containing a TypeExpr and an is_ref flag.
/// If is_ref is true, we apply the InOut reference modifier.
pub fn return_type_to_data_type(return_type: &ReturnType<'_>) -> DataType {
    // Check if the return type is void
    if matches!(return_type.ty.base, TypeBase::Primitive(AstPrimitiveType::Void)) {
        return DataType::simple(primitive_hashes::VOID);
    }

    // Convert the type expression
    let ref_modifier = if return_type.is_ref {
        RefModifier::InOut  // Return by reference
    } else {
        RefModifier::None
    };

    type_expr_to_data_type_with_ref(&return_type.ty, ref_modifier)
}

/// Convert a FunctionSignatureDecl to FfiFunctionDef.
///
/// This is used for both standalone functions and methods.
/// Note: Operator methods should be registered via `.operator()` or `.operator_raw()`
/// which handle operator behavior registration separately.
pub fn signature_to_ffi_function(
    sig: &FunctionSignatureDecl<'_>,
    native_fn: NativeFn,
) -> FfiFunctionDef {
    let name = sig.name.name.to_string();
    let params: Vec<FfiParam> = sig.params.iter().map(function_param_to_ffi).collect();
    let return_type = return_type_to_data_type(&sig.return_type);

    FfiFunctionDef::new(name)
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
        let dt = type_expr_to_data_type(&type_expr);

        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
    }

    #[test]
    fn const_primitive_type() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("const int", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert!(dt.is_const);
    }

    #[test]
    fn user_type_resolved() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("MyClass", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        // User types are now resolved immediately with TypeHash::from_name()
        assert_eq!(dt.type_hash, TypeHash::from_name("MyClass"));
    }

    #[test]
    fn handle_type() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("MyClass@", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        assert_eq!(dt.type_hash, TypeHash::from_name("MyClass"));
        assert!(dt.is_handle);
    }

    #[test]
    fn template_type_uses_self() {
        let arena = Bump::new();

        // Template instantiations in FFI method signatures use SELF
        let type_expr = Parser::type_expr("array<int>", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        // Template instantiations use SELF since actual instantiation happens at compile time
        assert_eq!(dt.type_hash, primitive_hashes::SELF);
    }

    #[test]
    fn function_param_conversion() {
        let arena = Bump::new();

        let sig = Parser::function_decl("void foo(int x, float y = 1.0)", &arena).unwrap();

        let param1 = function_param_to_ffi(&sig.params[0]);
        assert_eq!(param1.name, "x");
        assert_eq!(param1.data_type.type_hash, primitive_hashes::INT32);
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
        assert_eq!(param1.data_type.ref_modifier, RefModifier::In);

        let param2 = function_param_to_ffi(&sig.params[1]);
        assert_eq!(param2.data_type.ref_modifier, RefModifier::Out);

        let param3 = function_param_to_ffi(&sig.params[2]);
        assert_eq!(param3.data_type.ref_modifier, RefModifier::InOut);
    }

    #[test]
    fn signature_conversion() {
        let arena = Bump::new();

        let sig = Parser::function_decl("int getValue() const", &arena).unwrap();
        let ffi_func = signature_to_ffi_function(&sig, dummy_native_fn());

        assert_eq!(ffi_func.name, "getValue");
        assert!(ffi_func.params.is_empty());
        assert!(ffi_func.is_const());
        assert_eq!(ffi_func.return_type.type_hash, primitive_hashes::INT32);
    }

    #[test]
    fn void_return_type() {
        let arena = Bump::new();

        let sig = Parser::function_decl("void doSomething()", &arena).unwrap();
        let ffi_func = signature_to_ffi_function(&sig, dummy_native_fn());

        assert_eq!(ffi_func.return_type.type_hash, primitive_hashes::VOID);
    }

    #[test]
    fn scoped_type_name() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("Game::Entity", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        // Scoped types are resolved with their full qualified name
        assert_eq!(dt.type_hash, TypeHash::from_name("Game::Entity"));
    }

    #[test]
    fn ref_kind_to_modifier_conversion() {
        assert_eq!(ref_kind_to_modifier(RefKind::None), RefModifier::None);
        assert_eq!(ref_kind_to_modifier(RefKind::RefIn), RefModifier::In);
        assert_eq!(ref_kind_to_modifier(RefKind::RefOut), RefModifier::Out);
        assert_eq!(ref_kind_to_modifier(RefKind::RefInOut), RefModifier::InOut);
        assert_eq!(ref_kind_to_modifier(RefKind::Ref), RefModifier::InOut);
    }

    #[test]
    fn auto_type_uses_variable_param() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("auto", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        assert_eq!(dt.type_hash, primitive_hashes::VARIABLE_PARAM);
    }

    #[test]
    fn unknown_type_uses_variable_param() {
        let arena = Bump::new();

        let type_expr = Parser::type_expr("?", &arena).unwrap();
        let dt = type_expr_to_data_type(&type_expr);

        assert_eq!(dt.type_hash, primitive_hashes::VARIABLE_PARAM);
    }
}
