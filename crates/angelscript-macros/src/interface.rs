//! Implementation of the `#[angelscript::interface]` attribute macro.
//!
//! This macro transforms a Rust trait into an AngelScript interface definition.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemTrait, TraitItem, FnArg, ReturnType};

/// Parse interface attributes.
#[derive(Debug, Default)]
pub struct InterfaceAttrs {
    /// Override the AngelScript interface name.
    pub name: Option<String>,
}

impl InterfaceAttrs {
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

struct InterfaceAttrsParser(InterfaceAttrs);

impl syn::parse::Parse for InterfaceAttrsParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        InterfaceAttrs::parse(input).map(InterfaceAttrsParser)
    }
}

pub fn interface_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = match syn::parse::<InterfaceAttrsParser>(attr) {
        Ok(parser) => parser.0,
        Err(err) => return err.to_compile_error().into(),
    };

    let input = parse_macro_input!(item as ItemTrait);

    match interface_inner(&attrs, &input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn interface_inner(attrs: &InterfaceAttrs, input: &ItemTrait) -> syn::Result<TokenStream2> {
    let trait_name = &input.ident;
    let trait_vis = &input.vis;
    let trait_items = &input.items;

    // Determine the AngelScript interface name
    let as_name = attrs.name.clone().unwrap_or_else(|| trait_name.to_string());

    // Collect method signatures
    let methods = collect_method_signatures(trait_items)?;
    let method_tokens: Vec<_> = methods.iter().map(|m| {
        let name = &m.name;
        let is_const = m.is_const;
        let param_types = &m.param_types;
        let return_type = &m.return_type;

        quote! {
            ::angelscript_core::InterfaceMethodMeta {
                name: #name,
                is_const: #is_const,
                param_types: vec![#(::std::any::TypeId::of::<#param_types>()),*],
                return_type: ::std::any::TypeId::of::<#return_type>(),
            }
        }
    }).collect();

    // Generate the metadata function
    let meta_fn_name = syn::Ident::new(
        &format!("__as_{}_interface_meta", trait_name),
        trait_name.span(),
    );

    Ok(quote! {
        #trait_vis trait #trait_name {
            #(#trait_items)*
        }

        /// Get interface metadata for registration.
        #trait_vis fn #meta_fn_name() -> ::angelscript_core::InterfaceMeta {
            ::angelscript_core::InterfaceMeta {
                name: #as_name,
                type_hash: ::angelscript_core::TypeHash::from_name(#as_name),
                methods: vec![#(#method_tokens),*],
            }
        }
    })
}

struct MethodInfo {
    name: String,
    is_const: bool,
    param_types: Vec<syn::Type>,
    return_type: syn::Type,
}

fn collect_method_signatures(items: &[TraitItem]) -> syn::Result<Vec<MethodInfo>> {
    let mut methods = Vec::new();

    for item in items {
        if let TraitItem::Fn(method) = item {
            let sig = &method.sig;
            let name = sig.ident.to_string();

            // Check if method is const (has &self receiver)
            let is_const = sig.inputs.iter().any(|arg| {
                if let FnArg::Receiver(recv) = arg {
                    recv.reference.is_some() && recv.mutability.is_none()
                } else {
                    false
                }
            });

            // Collect parameter types (excluding self)
            let param_types: Vec<_> = sig.inputs.iter()
                .filter_map(|arg| {
                    if let FnArg::Typed(pat_type) = arg {
                        Some((*pat_type.ty).clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Get return type
            let return_type = match &sig.output {
                ReturnType::Default => syn::parse_quote!(()),
                ReturnType::Type(_, ty) => (**ty).clone(),
            };

            methods.push(MethodInfo {
                name,
                is_const,
                param_types,
                return_type,
            });
        }
    }

    Ok(methods)
}
