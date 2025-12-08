//! Implementation of the `#[angelscript::function]` attribute macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn, FnArg, ReturnType, Pat, Type, Attribute};

use crate::attrs::{
    FunctionAttrs, ParamAttrs, ReturnAttrs, ListPatternAttrs,
    RefModeAttr, ReturnModeAttr, ListPatternKind,
};

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
    let fn_attrs = &input.attrs;

    // Determine if this is a method (has self parameter)
    let is_method = fn_inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));

    // Naming strategy:
    // - Methods always keep original name (can't replace with struct in impl block)
    // - Free functions with `keep`: keep original name, use __meta suffix
    // - Free functions without `keep` (default): unit struct gets original name, impl is mangled
    let use_unit_struct = !is_method && !attrs.keep;

    // Extract parameter info for metadata (non-generic calling convention)
    // For generic functions, params come from #[param] attributes, not function signature
    let param_tokens: Vec<_> = if attrs.is_generic {
        // Generic calling convention: use empty params, metadata comes from #[param] attributes
        Vec::new()
    } else {
        let params = extract_params(fn_inputs)?;

        // Generate param tokens with defaults from #[default(...)] and template from #[template(...)] on each param
        params.iter().map(|p| {
            let name = &p.name;
            let ty = strip_reference(&p.ty);
            let default_value = match &p.default {
                Some(val) => quote! { Some(#val) },
                None => quote! { None },
            };
            let template_param = match &p.template_param {
                Some(param_name) => quote! { Some(#param_name) },
                None => quote! { None },
            };
            // if_handle_then_const only applies to generic calling convention
            // For non-generic params, it's always false

            quote! {
                ::angelscript_core::ParamMeta {
                    name: #name,
                    type_hash: <#ty as ::angelscript_core::Any>::type_hash(),
                    default_value: #default_value,
                    template_param: #template_param,
                    if_handle_then_const: false,
                }
            }
        }).collect()
    };

    // Parse #[param(...)] attributes for generic calling convention
    let param_attrs = ParamAttrs::from_attrs(fn_attrs)?;
    let generic_param_tokens = generate_generic_params(&param_attrs);

    // Parse #[returns(...)] attribute for return metadata
    let return_attrs = ReturnAttrs::from_attrs(fn_attrs)?;

    // Generate return meta from #[returns] attribute or defaults
    let return_meta_token = generate_return_meta(fn_output, &return_attrs);

    // Parse #[list_pattern(...)] attribute
    let list_pattern_attrs = ListPatternAttrs::from_attrs(fn_attrs)?;
    let list_pattern_token = generate_list_pattern(&list_pattern_attrs);

    // Generate behavior kind
    let behavior = generate_behavior(attrs);

    // Generate function traits
    let is_const = attrs.is_const;
    let is_property = attrs.is_property;
    let is_generic = attrs.is_generic;

    // Generate as_name from explicit name attribute
    let as_name_token = match &attrs.name {
        Some(name) => quote! { Some(#name) },
        None => quote! { None },
    };

    // Generate property name - explicit override or infer from get_/set_ prefix
    let property_name_token = if let Some(ref explicit_name) = attrs.property_name {
        quote! { Some(#explicit_name) }
    } else if is_property {
        let fn_name_str = fn_name.to_string();
        if let Some(name) = fn_name_str.strip_prefix("get_") {
            quote! { Some(#name) }
        } else if let Some(name) = fn_name_str.strip_prefix("set_") {
            quote! { Some(#name) }
        } else {
            quote! { None }
        }
    } else {
        quote! { None }
    };

    // Filter out our helper attributes from the output
    let filtered_attrs = filter_helper_attrs(fn_attrs);

    // Filter #[default] from parameter attributes in output
    let filtered_inputs = filter_param_attrs(fn_inputs);

    // Generate associated_type - for methods with self, use Self::type_hash()
    let associated_type_token = if is_method {
        quote! { Some(<Self as ::angelscript_core::Any>::type_hash()) }
    } else {
        quote! { None }
    };

    // Parse template params for template functions
    let template_params_tokens = if let Some(ref template_str) = attrs.template {
        // Parse template params like "<T>" or "<T, U>"
        let params: Vec<&str> = template_str
            .trim_start_matches('<')
            .trim_end_matches('>')
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        quote! { vec![#(#params),*] }
    } else {
        quote! { vec![] }
    };

    // Common metadata body
    let meta_body = quote! {
        ::angelscript_core::FunctionMeta {
            name: stringify!(#fn_name),
            as_name: #as_name_token,
            params: vec![#(#param_tokens),*],
            generic_params: vec![#(#generic_param_tokens),*],
            return_meta: #return_meta_token,
            is_method: #is_method,
            associated_type: #associated_type_token,
            behavior: #behavior,
            is_const: #is_const,
            is_property: #is_property,
            property_name: #property_name_token,
            is_generic: #is_generic,
            list_pattern: #list_pattern_token,
            template_params: #template_params_tokens,
        }
    };

    if use_unit_struct {
        // Rune pattern: unit struct gets original name, impl is mangled
        // Users can write: module.function(print)
        let mangled_fn_name = format_ident!("__as_fn__{}", fn_name);

        Ok(quote! {
            /// Unit struct for function metadata. Pass this to `Module::function()`.
            #[allow(non_camel_case_types)]
            #fn_vis struct #fn_name;

            impl ::angelscript_registry::HasFunctionMeta for #fn_name {
                fn __as_fn_meta() -> ::angelscript_core::FunctionMeta {
                    #meta_body
                }
            }

            #[doc(hidden)]
            #[allow(non_snake_case)]
            #(#filtered_attrs)*
            fn #mangled_fn_name #fn_generics(#filtered_inputs) #fn_output
            #fn_block
        })
    } else {
        // Keep pattern: original function stays callable, use __meta suffix
        // For methods (inside impl blocks), we generate a const fn pointer.
        // Function items coerce to fn pointers in const context, so:
        //   const len__meta: fn() -> FunctionMeta = Self::__len_meta_fn;
        // This gives us a true fn pointer that IntoFunctionMeta can accept.
        let meta_fn_name = format_ident!("__{}_meta_fn", fn_name);
        let meta_const_name = format_ident!("{}__meta", fn_name);

        Ok(quote! {
            #(#filtered_attrs)*
            #fn_vis fn #fn_name #fn_generics(#filtered_inputs) #fn_output
            #fn_block

            #[doc(hidden)]
            #[allow(non_snake_case)]
            fn #meta_fn_name() -> ::angelscript_core::FunctionMeta {
                #meta_body
            }

            /// Metadata constant. Pass to `Module::function()`.
            #[doc(hidden)]
            #[allow(non_upper_case_globals, non_snake_case)]
            #fn_vis const #meta_const_name: fn() -> ::angelscript_core::FunctionMeta = Self::#meta_fn_name;
        })
    }
}

/// Parameter info extracted from function inputs.
struct ParamInfo {
    name: String,
    ty: Box<Type>,
    default: Option<String>,
    template_param: Option<String>,
}

/// Extract parameter names, types, and defaults from function inputs.
fn extract_params(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> syn::Result<Vec<ParamInfo>> {
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

                // Look for #[default(...)] attribute on this parameter
                let default = extract_param_default(&pat_type.attrs)?;

                // Look for #[template("T")] attribute
                let template_param = extract_param_template(&pat_type.attrs)?;

                params.push(ParamInfo {
                    name,
                    ty: pat_type.ty.clone(),
                    default,
                    template_param,
                });
            }
        }
    }

    Ok(params)
}

/// Extract default value from #[default(...)] attribute on a parameter.
fn extract_param_default(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("default") {
            // Parse the default value - it's a string literal in parens
            let lit: syn::LitStr = attr.parse_args()?;
            return Ok(Some(lit.value()));
        }
    }
    Ok(None)
}

/// Extract template parameter name from #[template("T")] attribute on a parameter.
fn extract_param_template(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("template") {
            // Parse the template param - just a string literal
            let lit: syn::LitStr = attr.parse_args()?;
            return Ok(Some(lit.value()));
        }
    }
    Ok(None)
}

/// Generate the behavior kind token.
fn generate_behavior(attrs: &FunctionAttrs) -> TokenStream2 {
    use crate::attrs::FunctionKind;

    // Check for operator first
    if let Some(ref op_str) = attrs.operator {
        let op_variant = operator_path_to_variant(op_str);
        return quote! { Some(::angelscript_core::Behavior::Operator(#op_variant)) };
    }

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

/// Convert operator path string to token stream for Operator enum variant.
fn operator_path_to_variant(op_str: &str) -> TokenStream2 {
    // Handle paths like "Operator::Add" or just "Add"
    let variant = if op_str.contains("::") {
        op_str.split("::").last().unwrap_or(op_str)
    } else {
        op_str
    };

    let variant_ident = syn::Ident::new(variant, proc_macro2::Span::call_site());
    quote! { ::angelscript_core::Operator::#variant_ident }
}

/// Generate GenericParamMeta tokens from parsed #[param] attributes.
fn generate_generic_params(param_attrs: &[ParamAttrs]) -> Vec<TokenStream2> {
    param_attrs
        .iter()
        .map(|p| {
            let type_hash = if p.is_variable {
                quote! { ::angelscript_core::primitives::VARIABLE_PARAM }
            } else if let Some(ty) = &p.param_type {
                quote! { <#ty as ::angelscript_core::Any>::type_hash() }
            } else {
                // No type specified and not variable - default to VARIABLE_PARAM
                quote! { ::angelscript_core::primitives::VARIABLE_PARAM }
            };

            let ref_mode = match p.ref_mode {
                RefModeAttr::None => quote! { ::angelscript_core::RefModifier::None },
                RefModeAttr::In => quote! { ::angelscript_core::RefModifier::In },
                RefModeAttr::Out => quote! { ::angelscript_core::RefModifier::Out },
                RefModeAttr::InOut => quote! { ::angelscript_core::RefModifier::InOut },
            };

            let is_variadic = p.is_variadic;

            let default_value = match &p.default {
                Some(val) => quote! { Some(#val) },
                None => quote! { None },
            };

            let if_handle_then_const = p.if_handle_then_const;

            quote! {
                ::angelscript_core::GenericParamMeta {
                    type_hash: #type_hash,
                    ref_mode: #ref_mode,
                    is_variadic: #is_variadic,
                    default_value: #default_value,
                    if_handle_then_const: #if_handle_then_const,
                }
            }
        })
        .collect()
}

/// Generate ReturnMeta token from function output and #[returns] attribute.
fn generate_return_meta(fn_output: &ReturnType, return_attrs: &Option<ReturnAttrs>) -> TokenStream2 {
    match return_attrs {
        Some(attrs) => {
            // Get return type - explicit from attribute or from function signature
            let type_hash = if let Some(ty) = &attrs.return_type {
                quote! { Some(<#ty as ::angelscript_core::Any>::type_hash()) }
            } else {
                match fn_output {
                    ReturnType::Default => quote! { None },
                    ReturnType::Type(_, ty) => quote! { Some(<#ty as ::angelscript_core::Any>::type_hash()) },
                }
            };

            let mode = match attrs.mode {
                ReturnModeAttr::Value => quote! { ::angelscript_core::ReturnMode::Value },
                ReturnModeAttr::Reference => quote! { ::angelscript_core::ReturnMode::Reference },
                ReturnModeAttr::Handle => quote! { ::angelscript_core::ReturnMode::Handle },
            };

            let is_const = attrs.is_const;
            let is_variable = attrs.is_variable;

            quote! {
                ::angelscript_core::ReturnMeta {
                    type_hash: #type_hash,
                    mode: #mode,
                    is_const: #is_const,
                    is_variable: #is_variable,
                }
            }
        }
        None => {
            // Default: infer from function signature
            match fn_output {
                ReturnType::Default => quote! {
                    ::angelscript_core::ReturnMeta {
                        type_hash: None,
                        mode: ::angelscript_core::ReturnMode::Value,
                        is_const: false,
                        is_variable: false,
                    }
                },
                ReturnType::Type(_, ty) => quote! {
                    ::angelscript_core::ReturnMeta {
                        type_hash: Some(<#ty as ::angelscript_core::Any>::type_hash()),
                        mode: ::angelscript_core::ReturnMode::Value,
                        is_const: false,
                        is_variable: false,
                    }
                },
            }
        }
    }
}

/// Generate ListPatternMeta token from #[list_pattern] attribute.
fn generate_list_pattern(list_pattern_attrs: &Option<ListPatternAttrs>) -> TokenStream2 {
    match list_pattern_attrs {
        Some(attrs) => match &attrs.pattern {
            ListPatternKind::Repeat(ty) => {
                quote! {
                    Some(::angelscript_core::ListPatternMeta::Repeat(
                        <#ty as ::angelscript_core::Any>::type_hash()
                    ))
                }
            }
            ListPatternKind::Fixed(types) => {
                let type_tokens: Vec<_> = types
                    .iter()
                    .map(|ty| quote! { <#ty as ::angelscript_core::Any>::type_hash() })
                    .collect();
                quote! {
                    Some(::angelscript_core::ListPatternMeta::Fixed(
                        vec![#(#type_tokens),*]
                    ))
                }
            }
            ListPatternKind::RepeatTuple(types) => {
                let type_tokens: Vec<_> = types
                    .iter()
                    .map(|ty| quote! { <#ty as ::angelscript_core::Any>::type_hash() })
                    .collect();
                quote! {
                    Some(::angelscript_core::ListPatternMeta::RepeatTuple(
                        vec![#(#type_tokens),*]
                    ))
                }
            }
        },
        None => quote! { None },
    }
}

/// Filter out helper attributes (#[param], #[returns], #[list_pattern]) from output.
fn filter_helper_attrs(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attr| {
            let path = attr.path();
            !path.is_ident("param")
                && !path.is_ident("returns")
                && !path.is_ident("return")
                && !path.is_ident("list_pattern")
        })
        .collect()
}

/// Filter #[default] and #[template] attributes from function parameters.
fn filter_param_attrs(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> TokenStream2 {
    let filtered: Vec<_> = inputs.iter().map(|arg| {
        match arg {
            FnArg::Receiver(recv) => quote! { #recv },
            FnArg::Typed(pat_type) => {
                // Filter out #[default] and #[template] attributes
                let filtered_attrs: Vec<_> = pat_type.attrs.iter()
                    .filter(|attr| {
                        !attr.path().is_ident("default") && !attr.path().is_ident("template")
                    })
                    .collect();
                let pat = &pat_type.pat;
                let ty = &pat_type.ty;
                let colon = &pat_type.colon_token;
                quote! {
                    #(#filtered_attrs)*
                    #pat #colon #ty
                }
            }
        }
    }).collect();

    quote! { #(#filtered),* }
}

/// Strip reference from a type to get the underlying type.
/// `&T` -> `T`, `&mut T` -> `T`, `T` -> `T`
fn strip_reference(ty: &Type) -> TokenStream2 {
    match ty {
        Type::Reference(type_ref) => {
            let elem = &type_ref.elem;
            quote! { #elem }
        }
        _ => quote! { #ty },
    }
}
