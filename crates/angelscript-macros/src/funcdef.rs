//! Implementation of the `#[angelscript::funcdef]` attribute macro.
//!
//! This macro creates an AngelScript function pointer type (funcdef) from a type alias.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemType, ReturnType, Type, TypeBareFn, parse_macro_input};

/// Parse funcdef attributes.
#[derive(Debug, Default)]
pub struct FuncdefAttrs {
    /// Override the AngelScript funcdef name.
    pub name: Option<String>,
    /// Parent type for child funcdefs (e.g., `parent = ScriptArray`).
    pub parent: Option<syn::Type>,
    /// Explicit parameter types for template funcdefs (e.g., `params(T, T)`).
    /// When specified, uses VARIABLE_PARAM for template parameters.
    pub template_params: Vec<String>,
    /// Explicit return type for template funcdefs.
    pub template_return: Option<String>,
}

impl FuncdefAttrs {
    pub fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::{Token, punctuated::Punctuated};

        let mut result = Self::default();

        if input.is_empty() {
            return Ok(result);
        }

        // Parse comma-separated attributes: name = "...", parent = Type
        let items = Punctuated::<FuncdefAttrItem, Token![,]>::parse_terminated(input)?;

        for item in items {
            match item {
                FuncdefAttrItem::Name(name) => result.name = Some(name),
                FuncdefAttrItem::Parent(ty) => result.parent = Some(ty),
                FuncdefAttrItem::Params(params) => result.template_params = params,
                FuncdefAttrItem::Returns(ret) => result.template_return = Some(ret),
            }
        }

        Ok(result)
    }
}

/// Individual funcdef attribute item.
enum FuncdefAttrItem {
    Name(String),
    Parent(syn::Type),
    Params(Vec<String>),
    Returns(String),
}

impl syn::parse::Parse for FuncdefAttrItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::{LitStr, Token, parenthesized, punctuated::Punctuated};

        let ident: syn::Ident = input.parse()?;

        if ident == "name" {
            let _: Token![=] = input.parse()?;
            let value: LitStr = input.parse()?;
            Ok(FuncdefAttrItem::Name(value.value()))
        } else if ident == "parent" {
            let _: Token![=] = input.parse()?;
            let ty: syn::Type = input.parse()?;
            Ok(FuncdefAttrItem::Parent(ty))
        } else if ident == "params" {
            // Parse params(T, T) or params(bool)
            let content;
            parenthesized!(content in input);
            let items: Punctuated<syn::Ident, Token![,]> =
                content.parse_terminated(syn::Ident::parse, Token![,])?;
            let params: Vec<String> = items.iter().map(|i| i.to_string()).collect();
            Ok(FuncdefAttrItem::Params(params))
        } else if ident == "returns" {
            let _: Token![=] = input.parse()?;
            let ret: syn::Ident = input.parse()?;
            Ok(FuncdefAttrItem::Returns(ret.to_string()))
        } else {
            Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown funcdef attribute '{}'. Valid attributes are: name, parent, params, returns",
                    ident
                ),
            ))
        }
    }
}

struct FuncdefAttrsParser(FuncdefAttrs);

impl syn::parse::Parse for FuncdefAttrsParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        FuncdefAttrs::parse(input).map(FuncdefAttrsParser)
    }
}

pub fn funcdef_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<FuncdefAttrsParser>(attr) {
        Ok(parser) => parser.0,
        Err(err) => return err.to_compile_error().into(),
    };

    let input = parse_macro_input!(item as ItemType);

    match funcdef_inner(&attrs, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn funcdef_inner(attrs: &FuncdefAttrs, input: &ItemType) -> syn::Result<TokenStream2> {
    let type_name = &input.ident;
    let type_vis = &input.vis;
    let type_ty = &input.ty;

    // Determine the AngelScript funcdef name
    let as_name = attrs.name.clone().unwrap_or_else(|| type_name.to_string());

    // Check if we have explicit template params - if so, use them instead of extracting from fn type
    let (param_type_tokens, return_type_token) = if !attrs.template_params.is_empty() {
        // Use template params - single uppercase letter means VARIABLE_PARAM
        let param_tokens: Vec<_> = attrs
            .template_params
            .iter()
            .map(|p| {
                if p.len() == 1 && p.chars().next().unwrap().is_ascii_uppercase() {
                    // Template parameter like T, U, V - use VARIABLE_PARAM
                    quote! { ::angelscript_core::primitives::VARIABLE_PARAM }
                } else {
                    // Concrete type name - use TypeHash::from_name
                    quote! { ::angelscript_core::TypeHash::from_name(#p) }
                }
            })
            .collect();

        // Return type from template_return or default to void
        let ret_token = if let Some(ret) = &attrs.template_return {
            if ret.len() == 1 && ret.chars().next().unwrap().is_ascii_uppercase() {
                quote! { ::angelscript_core::primitives::VARIABLE_PARAM }
            } else {
                quote! { ::angelscript_core::TypeHash::from_name(#ret) }
            }
        } else {
            // Try to extract from fn type if available
            match type_ty.as_ref() {
                Type::BareFn(bare_fn) => {
                    let (_, return_type) = extract_fn_signature(bare_fn)?;
                    quote! { <#return_type as ::angelscript_core::Any>::type_hash() }
                }
                _ => quote! { ::angelscript_core::primitives::VOID },
            }
        };

        (param_tokens, ret_token)
    } else {
        // Extract function signature from the type
        let (param_types, return_type) = match type_ty.as_ref() {
            Type::BareFn(bare_fn) => extract_fn_signature(bare_fn)?,
            _ => {
                return Err(syn::Error::new_spanned(
                    type_ty,
                    "funcdef requires a bare function type like `fn(i32) -> bool`",
                ));
            }
        };

        let param_tokens: Vec<_> = param_types
            .iter()
            .map(|ty| {
                quote! { <#ty as ::angelscript_core::Any>::type_hash() }
            })
            .collect();

        let ret_token = quote! { <#return_type as ::angelscript_core::Any>::type_hash() };

        (param_tokens, ret_token)
    };

    // Generate parent_type token
    let parent_type_token = match &attrs.parent {
        Some(ty) => quote! { Some(<#ty as ::angelscript_core::Any>::type_hash()) },
        None => quote! { None },
    };

    // Generate the metadata function
    let meta_fn_name = syn::Ident::new(
        &format!("__as_{}_funcdef_meta", type_name),
        type_name.span(),
    );

    Ok(quote! {
        #type_vis type #type_name = #type_ty;

        /// Get funcdef metadata for registration.
        #[allow(non_snake_case)]
        #type_vis fn #meta_fn_name() -> ::angelscript_core::FuncdefMeta {
            ::angelscript_core::FuncdefMeta {
                name: #as_name,
                type_hash: ::angelscript_core::TypeHash::from_name(#as_name),
                param_types: vec![#(#param_type_tokens),*],
                return_type: #return_type_token,
                parent_type: #parent_type_token,
            }
        }
    })
}

fn extract_fn_signature(bare_fn: &TypeBareFn) -> syn::Result<(Vec<syn::Type>, syn::Type)> {
    // Extract parameter types
    let param_types: Vec<_> = bare_fn.inputs.iter().map(|arg| arg.ty.clone()).collect();

    // Extract return type
    let return_type = match &bare_fn.output {
        ReturnType::Default => syn::parse_quote!(()),
        ReturnType::Type(_, ty) => (**ty).clone(),
    };

    Ok((param_types, return_type))
}
