//! Implementation of the `#[angelscript::funcdef]` attribute macro.
//!
//! This macro creates an AngelScript function pointer type (funcdef) from a type alias.
//!
//! # Usage
//!
//! Basic funcdef:
//! ```ignore
//! #[funcdef]
//! type Callback = fn(i32) -> bool;
//! ```
//!
//! Template funcdef with params attribute:
//! ```ignore
//! // Use `_` to infer type from fn signature, single uppercase letter for template param
//! #[funcdef(parent = ScriptArray, params(T, T))]
//! type Less = fn(Dynamic, Dynamic) -> bool;
//!
//! // Mixed concrete and template params
//! #[funcdef(params(_, T))]
//! type Mixed = fn(i32, Dynamic) -> bool;
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{ItemType, ReturnType, Type, parse_macro_input};

/// A param spec - either infer from fn type (_) or a template param (T, U, etc.)
#[derive(Debug, Clone)]
pub(crate) enum ParamSpec {
    /// Infer type from the fn signature
    Infer,
    /// Template parameter (single uppercase letter like T, U, V)
    Template,
}

/// Parse funcdef attributes.
#[derive(Debug, Default)]
pub(crate) struct FuncdefAttrs {
    /// Override the AngelScript funcdef name.
    pub name: Option<String>,
    /// Parent type for child funcdefs (e.g., `parent = ScriptArray`).
    pub parent: Option<syn::Type>,
    /// Parameter specs: `_` for infer, single uppercase for template
    pub params: Vec<ParamSpec>,
}

impl FuncdefAttrs {
    pub fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::{Token, punctuated::Punctuated};

        let mut result = Self::default();

        if input.is_empty() {
            return Ok(result);
        }

        // Parse comma-separated attributes: name = "...", parent = Type, params(T, T)
        let items = Punctuated::<FuncdefAttrItem, Token![,]>::parse_terminated(input)?;

        for item in items {
            match item {
                FuncdefAttrItem::Name(name) => result.name = Some(name),
                FuncdefAttrItem::Parent(ty) => result.parent = Some(ty),
                FuncdefAttrItem::Params(params) => result.params = params,
            }
        }

        Ok(result)
    }
}

/// Individual funcdef attribute item.
enum FuncdefAttrItem {
    Name(String),
    Parent(syn::Type),
    Params(Vec<ParamSpec>),
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
            // Parse params(T, T) or params(_, T)
            let content;
            parenthesized!(content in input);

            let mut params = Vec::new();
            let items: Punctuated<proc_macro2::TokenTree, Token![,]> =
                content.parse_terminated(proc_macro2::TokenTree::parse, Token![,])?;

            for item in items {
                match item {
                    proc_macro2::TokenTree::Punct(p) if p.as_char() == '_' => {
                        params.push(ParamSpec::Infer);
                    }
                    proc_macro2::TokenTree::Ident(ident) => {
                        let name = ident.to_string();
                        // Single uppercase letter = template param
                        if name.len() == 1 && name.chars().next().unwrap().is_ascii_uppercase() {
                            params.push(ParamSpec::Template);
                        } else {
                            return Err(syn::Error::new(
                                ident.span(),
                                format!(
                                    "invalid param spec '{}'. Use `_` to infer from fn type, or single uppercase letter (T, U, V) for template param",
                                    name
                                ),
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "invalid param spec. Use `_` to infer from fn type, or single uppercase letter (T, U, V) for template param",
                        ));
                    }
                }
            }

            Ok(FuncdefAttrItem::Params(params))
        } else {
            Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown funcdef attribute '{}'. Valid attributes are: name, parent, params",
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

    // Extract function signature from the type
    let bare_fn = match type_ty.as_ref() {
        Type::BareFn(bare_fn) => bare_fn,
        _ => {
            return Err(syn::Error::new_spanned(
                type_ty,
                "funcdef requires a bare function type like `fn(i32) -> bool`",
            ));
        }
    };

    // Get parameter types from fn signature
    let fn_param_types: Vec<_> = bare_fn.inputs.iter().map(|arg| &arg.ty).collect();

    // Generate type hash tokens for each parameter
    let param_type_tokens: Vec<TokenStream2> = if attrs.params.is_empty() {
        // No params attribute - infer all from fn signature
        fn_param_types
            .iter()
            .map(|ty| quote! { <#ty as ::angelscript_core::Any>::type_hash() })
            .collect()
    } else {
        // Validate param count matches
        if attrs.params.len() != fn_param_types.len() {
            return Err(syn::Error::new_spanned(
                bare_fn,
                format!(
                    "params(...) has {} entries but fn type has {} parameters",
                    attrs.params.len(),
                    fn_param_types.len()
                ),
            ));
        }

        // Generate based on param specs
        attrs
            .params
            .iter()
            .zip(fn_param_types.iter())
            .map(|(spec, ty)| match spec {
                ParamSpec::Infer => {
                    quote! { <#ty as ::angelscript_core::Any>::type_hash() }
                }
                ParamSpec::Template => {
                    quote! { ::angelscript_core::primitives::VARIABLE_PARAM }
                }
            })
            .collect()
    };

    // Extract return type
    let return_type_token = match &bare_fn.output {
        ReturnType::Default => quote! { ::angelscript_core::primitives::VOID },
        ReturnType::Type(_, ty) => {
            quote! { <#ty as ::angelscript_core::Any>::type_hash() }
        }
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
        /// Funcdef handle type for AngelScript function pointers.
        ///
        /// This is an opaque handle - the actual function pointer is managed by the VM.
        #[repr(transparent)]
        #type_vis struct #type_name(::angelscript_core::FuncdefHandle);

        impl ::angelscript_core::Any for #type_name {
            fn type_hash() -> ::angelscript_core::TypeHash {
                ::angelscript_core::TypeHash::from_name(#as_name)
            }

            fn type_name() -> &'static str {
                #as_name
            }
        }

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
