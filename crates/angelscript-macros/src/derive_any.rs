//! Implementation of the `#[derive(Any)]` macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

use crate::attrs::{FieldAttrs, TypeAttrs, TypeKindAttr};

pub fn derive_any_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_any_inner(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_any_inner(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let attrs = TypeAttrs::from_attrs(&input.attrs)?;

    // Determine the AngelScript type name
    let as_name = attrs.name.clone().unwrap_or_else(|| name.to_string());

    // Generate the Any trait implementation
    let any_impl = generate_any_impl(name, &as_name);

    // Generate the type metadata function
    let type_meta = generate_type_meta(input, &attrs, &as_name)?;

    Ok(quote! {
        #any_impl
        #type_meta
    })
}

/// Generate the `Any` trait implementation.
fn generate_any_impl(name: &syn::Ident, as_name: &str) -> TokenStream2 {
    quote! {
        impl ::angelscript_core::Any for #name {
            fn type_hash() -> ::angelscript_core::TypeHash {
                ::angelscript_core::TypeHash::from_name(#as_name)
            }

            fn type_name() -> &'static str {
                #as_name
            }
        }
    }
}

/// Generate the `HasClassMeta` trait impl for collecting type metadata.
fn generate_type_meta(
    input: &DeriveInput,
    attrs: &TypeAttrs,
    as_name: &str,
) -> syn::Result<TokenStream2> {
    let name = &input.ident;

    // Parse type kind
    let type_kind_tokens = match attrs.type_kind {
        Some(TypeKindAttr::Value) => quote! { ::angelscript_core::TypeKind::value::<#name>() },
        Some(TypeKindAttr::Pod) => quote! { ::angelscript_core::TypeKind::pod::<#name>() },
        Some(TypeKindAttr::Reference) => quote! { ::angelscript_core::TypeKind::reference() },
        Some(TypeKindAttr::Scoped) => quote! { ::angelscript_core::TypeKind::scoped() },
        Some(TypeKindAttr::NoCount) => quote! { ::angelscript_core::TypeKind::single_ref() },
        Some(TypeKindAttr::AsHandle) => quote! { ::angelscript_core::TypeKind::generic_handle() },
        None => quote! { ::angelscript_core::TypeKind::reference() },
    };

    // Parse template params if present
    let template_tokens = if let Some(ref template) = attrs.template {
        // Parse template params like "<T>" or "<K, V>"
        let params: Vec<&str> = template
            .trim_start_matches('<')
            .trim_end_matches('>')
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let param_names: Vec<&str> = params.clone();

        quote! {
            template_params: vec![#(#param_names),*],
        }
    } else {
        quote! { template_params: vec![], }
    };

    // Generate specialization fields
    let specialization_of_token = match &attrs.specialization_of {
        Some(base_name) => quote! { Some(#base_name) },
        None => quote! { None },
    };

    let specialization_args_tokens: Vec<_> = attrs
        .specialization_args
        .iter()
        .map(|ty| {
            quote! { <#ty as ::angelscript_core::Any>::type_hash() }
        })
        .collect();

    // Collect property metadata from fields
    let properties = collect_properties(input)?;

    Ok(quote! {
        impl ::angelscript_registry::HasClassMeta for #name {
            fn __as_type_meta() -> ::angelscript_core::ClassMeta {
                ::angelscript_core::ClassMeta {
                    name: #as_name,
                    type_hash: <#name as ::angelscript_core::Any>::type_hash(),
                    type_kind: #type_kind_tokens,
                    properties: vec![#(#properties),*],
                    #template_tokens
                    specialization_of: #specialization_of_token,
                    specialization_args: vec![#(#specialization_args_tokens),*],
                }
            }
        }
    })
}

/// Collect property metadata from struct fields.
fn collect_properties(input: &DeriveInput) -> syn::Result<Vec<TokenStream2>> {
    let mut properties = Vec::new();

    if let Data::Struct(data) = &input.data
        && let Fields::Named(fields) = &data.fields
    {
        for field in &fields.named {
            let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;

            // Only include fields with get or set attributes
            if !field_attrs.get && !field_attrs.set {
                continue;
            }

            let field_name = field.ident.as_ref().unwrap();
            let prop_name = field_attrs
                .name
                .clone()
                .unwrap_or_else(|| field_name.to_string());
            let field_ty = &field.ty;

            let get = field_attrs.get;
            let set = field_attrs.set;

            properties.push(quote! {
                ::angelscript_core::PropertyMeta {
                    name: #prop_name,
                    get: #get,
                    set: #set,
                    type_hash: <#field_ty as ::angelscript_core::Any>::type_hash(),
                }
            });
        }
    }

    Ok(properties)
}
