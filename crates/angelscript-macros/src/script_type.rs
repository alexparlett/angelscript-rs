//! Derive macro for `ScriptType` — generates type metadata for script registration.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields, Lit, Meta};

/// Parse the `#[script(name = "...")]` attribute from a struct-level attribute list.
fn parse_script_name(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("script") {
            continue;
        }
        let mut found_name = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(lit_str) = lit {
                    found_name = Some(lit_str.value());
                }
            }
            Ok(())
        });
        if found_name.is_some() {
            return found_name;
        }
    }
    None
}

/// Check if a field has `#[script(property)]`.
fn is_script_property(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if !attr.path().is_ident("script") {
            continue;
        }
        // Try parsing as Meta::List with a path inside.
        if let Meta::List(_) = &attr.meta {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("property") {
                    found = true;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

/// Map a Rust type path to its AngelScript type name string.
fn rust_type_to_script_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            return match ident.as_str() {
                "bool" => "bool".to_string(),
                "i8" => "int8".to_string(),
                "i16" => "int16".to_string(),
                "i32" => "int".to_string(),
                "i64" => "int64".to_string(),
                "u8" => "uint8".to_string(),
                "u16" => "uint16".to_string(),
                "u32" => "uint".to_string(),
                "u64" => "uint64".to_string(),
                "f32" => "float".to_string(),
                "f64" => "double".to_string(),
                "String" => "string".to_string(),
                other => other.to_string(),
            };
        }
    }
    "unknown".to_string()
}

pub fn derive_script_type_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Determine script-visible name.
    let script_name = parse_script_name(&input.attrs).unwrap_or_else(|| struct_name.to_string());

    // Collect properties from fields.
    let fields = match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return syn::Error::new_spanned(
                    struct_name,
                    "ScriptType can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "ScriptType can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut property_names = Vec::new();
    let mut property_script_types = Vec::new();

    for field in fields {
        if is_script_property(field) {
            let field_name = field.ident.as_ref().unwrap().to_string();
            let script_type_name = rust_type_to_script_name(&field.ty);
            property_names.push(field_name);
            property_script_types.push(script_type_name);
        }
    }

    let property_count = property_names.len();

    let expanded = quote! {
        impl #struct_name {
            /// The script-visible name for this type.
            pub const SCRIPT_NAME: &'static str = #script_name;

            /// Property metadata: (name, script_type_name) pairs.
            pub fn script_properties() -> Vec<(&'static str, &'static str)> {
                vec![
                    #( (#property_names, #property_script_types), )*
                ]
            }

            /// Number of script-exposed properties.
            pub fn script_property_count() -> usize {
                #property_count
            }
        }
    };

    expanded.into()
}
