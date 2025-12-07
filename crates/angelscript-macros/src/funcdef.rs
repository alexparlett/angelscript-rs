//! Implementation of the `#[angelscript::funcdef]` attribute macro.
//!
//! This macro creates an AngelScript function pointer type (funcdef) from a type alias.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemType, Type, TypeBareFn, ReturnType};

/// Parse funcdef attributes.
#[derive(Debug, Default)]
pub struct FuncdefAttrs {
    /// Override the AngelScript funcdef name.
    pub name: Option<String>,
}

impl FuncdefAttrs {
    pub fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::{Token, LitStr};

        let mut result = Self::default();

        if input.is_empty() {
            return Ok(result);
        }

        // Parse name = "..."
        if input.peek(syn::Ident) {
            let ident: syn::Ident = input.parse()?;
            if ident == "name" {
                let _: Token![=] = input.parse()?;
                let value: LitStr = input.parse()?;
                result.name = Some(value.value());
            }
        }

        Ok(result)
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
    let (param_types, return_type) = match type_ty.as_ref() {
        Type::BareFn(bare_fn) => extract_fn_signature(bare_fn)?,
        _ => {
            return Err(syn::Error::new_spanned(
                type_ty,
                "funcdef requires a bare function type like `fn(i32) -> bool`",
            ));
        }
    };

    let param_type_tokens: Vec<_> = param_types.iter().map(|ty| {
        quote! { ::std::any::TypeId::of::<#ty>() }
    }).collect();

    // Generate the metadata function
    let meta_fn_name = syn::Ident::new(
        &format!("__as_{}_funcdef_meta", type_name),
        type_name.span(),
    );

    Ok(quote! {
        #type_vis type #type_name = #type_ty;

        /// Get funcdef metadata for registration.
        #type_vis fn #meta_fn_name() -> ::angelscript_core::FuncdefMeta {
            ::angelscript_core::FuncdefMeta {
                name: #as_name,
                type_hash: ::angelscript_core::TypeHash::from_name(#as_name),
                param_types: vec![#(#param_type_tokens),*],
                return_type: ::std::any::TypeId::of::<#return_type>(),
            }
        }
    })
}

fn extract_fn_signature(bare_fn: &TypeBareFn) -> syn::Result<(Vec<syn::Type>, syn::Type)> {
    // Extract parameter types
    let param_types: Vec<_> = bare_fn.inputs.iter()
        .map(|arg| arg.ty.clone())
        .collect();

    // Extract return type
    let return_type = match &bare_fn.output {
        ReturnType::Default => syn::parse_quote!(()),
        ReturnType::Type(_, ty) => (**ty).clone(),
    };

    Ok((param_types, return_type))
}
