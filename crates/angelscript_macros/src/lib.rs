extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, FnArg, ItemFn, Pat};

#[proc_macro_derive(Generic)]
pub fn derive_from_script_generic(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Only struct with named fields supported
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => panic!("FromScriptGeneric only on named fields"),
        },
        _ => panic!("FromScriptGeneric only on structs"),
    };

    let field_inits = fields.iter().enumerate().map(|(i, f)| {
        let fname = &f.ident;
        let fty = &f.ty;
        quote! {
            #fname: <#fty as crate::FromScriptGeneric>::from_script_generic(ctx, arg_idx + #i as u32)
        }
    });

    let expanded = quote! {
        impl crate::FromScriptGeneric for #name {
            fn from_script_generic(ctx: *mut crate::asIScriptGeneric, arg_idx: u32) -> Self {
                #name {
                    #(#field_inits),*
                }
            }
        }
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn as_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let wrapper_name = syn::Ident::new(&format!("{}_as_generic", fn_name), fn_name.span());

    let mut arg_vars = Vec::new();
    let mut arg_names = Vec::new();
    for (i, arg) in input_fn.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(pt) = arg {
            let name = match &*pt.pat {
                Pat::Ident(id) => &id.ident,
                _ => panic!("Unsupported arg pattern"),
            };
            let ty = &*pt.ty;
            arg_vars.push(quote! {
                let #name = <#ty as crate::FromScriptGeneric>::from_script_generic(ctx, #i as u32);
            });
            arg_names.push(quote! { #name });
        }
    }

    let call = quote! { #fn_name(#(#arg_names),*) };
    let ret_block = match &input_fn.sig.output {
        syn::ReturnType::Default => quote! { #call; },
        syn::ReturnType::Type(_, ret_ty) => {
            // Simple specialization for primitive returns for illustration
            quote! {
                let ret = #call;
                crate::set_return::<#ret_ty>(ctx, ret);
            }
        }
    };

    let orig = quote! { #input_fn };

    let wrapper = quote! {

        #[unsafe(no_mangle)]
        pub extern "C" fn #wrapper_name(ctx: *mut crate::asIScriptGeneric) {
            #(#arg_vars)*
            #ret_block
        }
    };
    quote!( #orig #wrapper ).into()
}
