//! Implementation of the `#[angelscript::function]` attribute macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn, FnArg, ReturnType, Pat, Type};

use crate::attrs::FunctionAttrs;

pub fn function_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<FunctionAttrsParser>(attr) {
        Ok(parser) => parser.0,
        Err(err) => return err.to_compile_error().into(),
    };

    let input = parse_macro_input!(item as ItemFn);

    match function_inner(&attrs, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Wrapper to parse FunctionAttrs
struct FunctionAttrsParser(FunctionAttrs);

impl syn::parse::Parse for FunctionAttrsParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        FunctionAttrs::parse(input).map(FunctionAttrsParser)
    }
}

fn function_inner(attrs: &FunctionAttrs, input: &ItemFn) -> syn::Result<TokenStream2> {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_generics = &input.sig.generics;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;

    // Generate the metadata function name
    let meta_fn_name = syn::Ident::new(
        &format!("__as_{}_meta", fn_name),
        fn_name.span(),
    );

    // Determine if this is a method (has self parameter)
    let is_method = fn_inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));

    // Extract parameter info for metadata
    let params = extract_params(fn_inputs)?;
    let param_tokens: Vec<_> = params.iter().map(|(name, ty)| {
        quote! {
            ::angelscript_core::ParamMeta {
                name: #name,
                rust_type: ::std::any::TypeId::of::<#ty>(),
            }
        }
    }).collect();

    // Determine return type
    let return_type_token = match fn_output {
        ReturnType::Default => quote! { ::std::any::TypeId::of::<()>() },
        ReturnType::Type(_, ty) => quote! { ::std::any::TypeId::of::<#ty>() },
    };

    // Generate behavior kind
    let behavior = generate_behavior(attrs);

    // Generate function traits
    let is_const = attrs.is_const;
    let is_property = attrs.is_property;

    // Output the original function plus the metadata generator
    Ok(quote! {
        #fn_vis fn #fn_name #fn_generics(#fn_inputs) #fn_output
        #fn_block

        #[doc(hidden)]
        #fn_vis fn #meta_fn_name() -> ::angelscript_core::FunctionMeta {
            ::angelscript_core::FunctionMeta {
                name: stringify!(#fn_name),
                params: vec![#(#param_tokens),*],
                return_type: #return_type_token,
                is_method: #is_method,
                behavior: #behavior,
                is_const: #is_const,
                is_property: #is_property,
            }
        }
    })
}

/// Extract parameter names and types from function inputs.
fn extract_params(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> syn::Result<Vec<(String, Box<Type>)>> {
    let mut params = Vec::new();

    for arg in inputs {
        match arg {
            FnArg::Receiver(_) => {
                // Skip self parameter
            }
            FnArg::Typed(pat_type) => {
                let name = match pat_type.pat.as_ref() {
                    Pat::Ident(ident) => ident.ident.to_string(),
                    _ => "_".to_string(),
                };
                params.push((name, pat_type.ty.clone()));
            }
        }
    }

    Ok(params)
}

/// Generate the behavior kind token.
fn generate_behavior(attrs: &FunctionAttrs) -> TokenStream2 {
    use crate::attrs::FunctionKind;

    match attrs.kind {
        FunctionKind::Global => quote! { None },
        FunctionKind::Instance => quote! { None },
        FunctionKind::Constructor => {
            if attrs.is_copy {
                quote! { Some(::angelscript_core::Behavior::CopyConstructor) }
            } else {
                quote! { Some(::angelscript_core::Behavior::Constructor) }
            }
        }
        FunctionKind::Factory => quote! { Some(::angelscript_core::Behavior::Factory) },
        FunctionKind::Destructor => quote! { Some(::angelscript_core::Behavior::Destructor) },
        FunctionKind::AddRef => quote! { Some(::angelscript_core::Behavior::AddRef) },
        FunctionKind::Release => quote! { Some(::angelscript_core::Behavior::Release) },
        FunctionKind::ListConstruct => quote! { Some(::angelscript_core::Behavior::ListConstruct) },
        FunctionKind::ListFactory => quote! { Some(::angelscript_core::Behavior::ListFactory) },
        FunctionKind::TemplateCallback => quote! { Some(::angelscript_core::Behavior::TemplateCallback) },
        FunctionKind::GcGetRefCount => quote! { Some(::angelscript_core::Behavior::GcGetRefCount) },
        FunctionKind::GcSetFlag => quote! { Some(::angelscript_core::Behavior::GcSetFlag) },
        FunctionKind::GcGetFlag => quote! { Some(::angelscript_core::Behavior::GcGetFlag) },
        FunctionKind::GcEnumRefs => quote! { Some(::angelscript_core::Behavior::GcEnumRefs) },
        FunctionKind::GcReleaseRefs => quote! { Some(::angelscript_core::Behavior::GcReleaseRefs) },
        FunctionKind::GetWeakRefFlag => quote! { Some(::angelscript_core::Behavior::GetWeakRefFlag) },
    }
}
