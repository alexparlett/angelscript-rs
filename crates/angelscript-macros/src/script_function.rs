//! Attribute macro for registering free functions as native script functions.
//!
//! Generates a registration helper that creates a `NativeFunction` wrapping the
//! Rust function, adapting arguments from `NativeCallContext` and writing back
//! the return value.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, ReturnType, Type};

/// Determine how to read a parameter of the given type from NativeCallContext.
/// Returns a token stream that reads arg at `index`.
fn arg_reader(ty: &Type, index: usize) -> proc_macro2::TokenStream {
    let type_name = type_ident_string(ty);
    match type_name.as_str() {
        "i32" => quote! { ctx.arg_i32(#index) },
        "u32" => quote! { ctx.args.get(#index).copied().unwrap_or(0) },
        "f32" => quote! { ctx.arg_f32(#index) },
        "bool" => quote! { ctx.args.get(#index).copied().unwrap_or(0) != 0 },
        "i64" => {
            let lo = index;
            let hi = index + 1;
            quote! {
                {
                    let lo = ctx.args.get(#lo).copied().unwrap_or(0) as u64;
                    let hi = ctx.args.get(#hi).copied().unwrap_or(0) as u64;
                    ((hi << 32) | lo) as i64
                }
            }
        }
        "u64" => {
            let lo = index;
            let hi = index + 1;
            quote! {
                {
                    let lo = ctx.args.get(#lo).copied().unwrap_or(0) as u64;
                    let hi = ctx.args.get(#hi).copied().unwrap_or(0) as u64;
                    (hi << 32) | lo
                }
            }
        }
        "f64" => {
            let lo = index;
            let hi = index + 1;
            quote! {
                {
                    let lo = ctx.args.get(#lo).copied().unwrap_or(0) as u64;
                    let hi = ctx.args.get(#hi).copied().unwrap_or(0) as u64;
                    f64::from_bits((hi << 32) | lo)
                }
            }
        }
        _ => quote! { compile_error!("Unsupported parameter type for #[script_function]") },
    }
}

/// Number of u32 stack slots a type consumes.
fn type_stack_slots(ty: &Type) -> usize {
    let name = type_ident_string(ty);
    match name.as_str() {
        "i64" | "u64" | "f64" => 2,
        _ => 1,
    }
}

/// Generate code to write the return value into the NativeCallContext.
fn return_writer(ty: &Type) -> proc_macro2::TokenStream {
    let type_name = type_ident_string(ty);
    match type_name.as_str() {
        "i32" => quote! { ctx.set_return_i32(result); },
        "u32" => {
            quote! {
                ctx.return_value = result as u64;
                ctx.return_is_64bit = false;
            }
        }
        "f32" => quote! { ctx.set_return_f32(result); },
        "bool" => {
            quote! {
                ctx.return_value = if result { 1 } else { 0 };
                ctx.return_is_64bit = false;
            }
        }
        "i64" => quote! { ctx.set_return_i64(result); },
        "u64" => {
            quote! {
                ctx.return_value = result;
                ctx.return_is_64bit = true;
            }
        }
        "f64" => quote! { ctx.set_return_f64(result); },
        _ => quote! { compile_error!("Unsupported return type for #[script_function]") },
    }
}

/// Map a Rust type to its AngelScript type name string.
fn rust_type_to_script_name(ty: &Type) -> String {
    let name = type_ident_string(ty);
    match name.as_str() {
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
        _ => name,
    }
}

/// Get the last path segment of a type as a string.
fn type_ident_string(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.to_string();
        }
    }
    String::new()
}

pub fn script_function_impl(item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;
    let func_name_str = func_name.to_string();
    let register_fn_name = format_ident!("register_{}", func_name);

    // Build argument readers.
    let mut arg_readers = Vec::new();
    let mut arg_names = Vec::new();
    let mut param_script_types = Vec::new();
    let mut slot_offset: usize = 0;

    for param in &func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = param {
            let arg_name = if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                pat_ident.ident.clone()
            } else {
                format_ident!("_arg")
            };

            let reader = arg_reader(&pat_type.ty, slot_offset);
            slot_offset += type_stack_slots(&pat_type.ty);

            arg_readers.push(reader);
            arg_names.push(arg_name);
            param_script_types.push(rust_type_to_script_name(&pat_type.ty));
        }
    }

    let _param_count = func.sig.inputs.len();

    // Determine return type handling.
    let has_return = !matches!(&func.sig.output, ReturnType::Default);
    let call_and_store = if has_return {
        let ret_ty = match &func.sig.output {
            ReturnType::Type(_, ty) => ty.as_ref(),
            _ => unreachable!(),
        };
        let writer = return_writer(ret_ty);
        quote! {
            let result = #func_name(#(#arg_names),*);
            #writer
        }
    } else {
        quote! {
            #func_name(#(#arg_names),*);
        }
    };

    let _return_script_type = match &func.sig.output {
        ReturnType::Default => "void".to_string(),
        ReturnType::Type(_, ty) => rust_type_to_script_name(ty),
    };

    let expanded = quote! {
        // Keep the original function.
        #func

        /// Register this function as a native script function in a module.
        ///
        /// Returns a `NativeFunction` ready to be added to a `Module`.
        pub fn #register_fn_name(
            id: angelscript_core::FunctionId,
        ) -> angelscript_vm::module::NativeFunction {
            angelscript_vm::module::NativeFunction {
                id,
                name: angelscript_core::QualifiedName::global(#func_name_str),
                callback: Box::new(|ctx: &mut angelscript_vm::module::NativeCallContext| {
                    #(let #arg_names = #arg_readers;)*
                    #call_and_store
                    Ok(())
                }),
            }
        }

        // Metadata constants for the registered function.
        #[allow(non_upper_case_globals)]
        const _: () = {
            // Static metadata accessible for introspection.
        };
    };

    expanded.into()
}
