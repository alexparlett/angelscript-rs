//! Implementation of the `#[angelscript::function]` attribute macro.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Attribute, FnArg, ItemFn, Pat, ReturnType, Type, parse_macro_input};

use crate::attrs::{
    FunctionAttrs, ListPatternAttrs, ListPatternKind, ParamAttrs, RefModeAttr, ReturnAttrs,
    ReturnModeAttr,
};

/// Check if a type string represents a primitive type that is passed by value.
/// Primitive types are copied rather than borrowed, so they don't cause borrow conflicts.
fn is_primitive_type(type_str: &str) -> bool {
    matches!(
        type_str,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "isize"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
    )
}

/// Check if a type string is a primitive integer type.
fn is_primitive_integer(type_str: &str) -> bool {
    matches!(
        type_str,
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"
    )
}

/// Check if a type string is a primitive float type.
fn is_primitive_float(type_str: &str) -> bool {
    matches!(type_str, "f32" | "f64")
}

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
    let is_method = fn_inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(_)));

    // Determine if we're in an impl block:
    // - Methods (with self) are always in impl blocks
    // - Explicit `keep` attribute indicates impl block
    // - Non-Global function kinds (instance, behaviors) imply impl block
    let in_impl_block = is_method || attrs.keep || attrs.kind.implies_impl_block();

    // Naming strategy:
    // - Functions in impl blocks: keep original name, use __meta suffix
    // - Free functions (default): unit struct gets original name, impl is mangled
    let use_unit_struct = !in_impl_block;

    // Extract parameter info for metadata (non-generic calling convention)
    // For generic functions, params come from #[param] attributes, not function signature
    let param_tokens: Vec<_> = if attrs.is_generic {
        // Generic calling convention: use empty params, metadata comes from #[param] attributes
        Vec::new()
    } else {
        let params = extract_params(fn_inputs)?;

        // Generate param tokens with defaults from #[default(...)] and template from #[template(...)] on each param
        let mut tokens = Vec::with_capacity(params.len());
        for p in &params {
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

            // For template params, use SELF as placeholder - resolved at instantiation
            let type_hash = if p.template_param.is_some() {
                quote! { ::angelscript_core::primitives::SELF }
            } else {
                quote! { <#ty as ::angelscript_core::Any>::type_hash() }
            };

            // Determine ref_mode with validation
            let ref_mode_token = match p.ref_mode {
                RefModeAttr::Out => {
                    if !p.is_mut_ref {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "`#[param(out)]` on parameter `{}` requires `&mut` type",
                                name
                            ),
                        ));
                    }
                    quote! { ::angelscript_core::RefModifier::Out }
                }
                RefModeAttr::InOut => {
                    if !p.is_mut_ref {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "`#[param(inout)]` on parameter `{}` requires `&mut` type",
                                name
                            ),
                        ));
                    }
                    quote! { ::angelscript_core::RefModifier::InOut }
                }
                RefModeAttr::In => {
                    if p.is_mut_ref {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "`#[param(in)]` on parameter `{}` cannot be used with `&mut` - use `out` or `inout` instead",
                                name
                            ),
                        ));
                    }
                    quote! { ::angelscript_core::RefModifier::In }
                }
                RefModeAttr::None => {
                    if p.is_mut_ref {
                        return Err(syn::Error::new(
                            proc_macro2::Span::call_site(),
                            format!(
                                "`&mut` parameter `{}` must have `#[param(out)]` or `#[param(inout)]` to specify AngelScript reference mode",
                                name
                            ),
                        ));
                    } else if p.is_ref {
                        quote! { ::angelscript_core::RefModifier::In }
                    } else {
                        quote! { ::angelscript_core::RefModifier::None }
                    }
                }
            };

            tokens.push(quote! {
                ::angelscript_core::ParamMeta {
                    name: #name,
                    type_hash: #type_hash,
                    default_value: #default_value,
                    template_param: #template_param,
                    if_handle_then_const: false,
                    ref_mode: #ref_mode_token,
                }
            });
        }
        tokens
    };

    // Generate function traits (early, needed for return meta)
    let is_const = attrs.is_const;
    let is_property = attrs.is_property;
    let is_generic = attrs.is_generic;

    // Check if this is a "true" generic calling convention function (takes &mut CallContext)
    // vs a "metadata-only" generic function (regular signature but uses generic_params for metadata)
    let is_generic_calling_convention = is_generic
        && fn_inputs.len() == 1
        && fn_inputs.iter().any(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                let ty_str = quote!(#pat_type.ty).to_string();
                ty_str.contains("CallContext")
            } else {
                false
            }
        });

    // Parse #[param(...)] attributes for generic calling convention
    let param_attrs = ParamAttrs::from_attrs(fn_attrs)?;
    let generic_param_tokens = generate_generic_params(&param_attrs);

    // Parse #[returns(...)] attribute for return metadata
    let return_attrs = ReturnAttrs::from_attrs(fn_attrs)?;

    // Generate return meta from #[returns] attribute or defaults
    // For generic calling convention functions, ignore the Rust return type (Result<(), NativeError>)
    let return_meta_token =
        generate_return_meta(fn_output, &return_attrs, is_generic_calling_convention);

    // Parse #[list_pattern(...)] attribute
    let list_pattern_attrs = ListPatternAttrs::from_attrs(fn_attrs)?;
    let list_pattern_token = generate_list_pattern(&list_pattern_attrs);

    // Generate behavior kind
    let behavior = generate_behavior(attrs);

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

    // Generate associated_type - for functions in impl blocks, use Self::type_hash()
    let associated_type_token = if in_impl_block {
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

    // Helper to generate the meta body with a specific native_fn token
    let generate_meta_body = |native_fn_token: TokenStream2| {
        quote! {
            ::angelscript_core::FunctionMeta {
                name: stringify!(#fn_name),
                as_name: #as_name_token,
                native_fn: #native_fn_token,
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
        }
    };

    if use_unit_struct {
        // Rune pattern: unit struct gets original name, impl is mangled
        // Users can write: module.function(print)
        let mangled_fn_name = format_ident!("__as_fn__{}", fn_name);

        // Generate NativeFn for free function
        let fn_name_str = fn_name.to_string();
        let native_fn_token = generate_native_fn(
            &fn_name_str,
            &mangled_fn_name,
            fn_inputs,
            fn_output,
            is_generic,
            true, // is_unit_struct
        );
        let meta_body = generate_meta_body(native_fn_token);

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

        // Generate NativeFn for method
        let fn_name_str = fn_name.to_string();
        let native_fn_token = generate_native_fn(
            &fn_name_str,
            fn_name,
            fn_inputs,
            fn_output,
            is_generic,
            false,
        );
        let meta_body = generate_meta_body(native_fn_token);

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
    /// Reference mode for the parameter (from `#[param(in/out/inout)]`)
    ref_mode: RefModeAttr,
    /// Whether the Rust type is `&mut T`
    is_mut_ref: bool,
    /// Whether the Rust type is `&T`
    is_ref: bool,
}

/// Extract parameter names, types, and defaults from function inputs.
fn extract_params(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> syn::Result<Vec<ParamInfo>> {
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

                // Check if type is &mut T or &T
                let (is_mut_ref, is_ref) = check_reference_type(&pat_type.ty);

                // Look for #[default(...)] attribute on this parameter
                let default = extract_param_default(&pat_type.attrs)?;

                // Look for #[template("T")] attribute
                let template_param = extract_param_template(&pat_type.attrs)?;

                // Look for #[param(...)] attribute on this parameter
                let param_attr = extract_param_attr(&pat_type.attrs)?;

                // Get ref_mode from #[param(...)] or use default
                let ref_mode = param_attr.as_ref().map(|p| p.ref_mode).unwrap_or_default();

                // Use default from #[param(default = "...")] if not set by #[default(...)]
                let default =
                    default.or_else(|| param_attr.as_ref().and_then(|p| p.default.clone()));

                // Use template from #[param(template = "...")] if not set by #[template(...)]
                let template_param = template_param
                    .or_else(|| param_attr.as_ref().and_then(|p| p.template_param.clone()));

                params.push(ParamInfo {
                    name,
                    ty: pat_type.ty.clone(),
                    default,
                    template_param,
                    ref_mode,
                    is_mut_ref,
                    is_ref,
                });
            }
        }
    }

    Ok(params)
}

/// Check if a type is a reference type and whether it's mutable.
fn check_reference_type(ty: &Type) -> (bool, bool) {
    match ty {
        Type::Reference(ref_type) => {
            let is_mut = ref_type.mutability.is_some();
            (is_mut, true)
        }
        _ => (false, false),
    }
}

/// Extract #[param(...)] attribute from a parameter's attributes.
/// Returns the parsed ParamAttrs if present.
fn extract_param_attr(attrs: &[Attribute]) -> syn::Result<Option<ParamAttrs>> {
    for attr in attrs {
        if attr.path().is_ident("param") {
            let param_attrs = ParamAttrs::from_attr(attr)?;

            // Validate that only parameter-level options are used
            if param_attrs.is_variable {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[param(variable)]` is only valid on function-level for generic calling convention, not on individual parameters",
                ));
            }
            if param_attrs.is_variadic {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[param(variadic)]` is only valid on function-level for generic calling convention, not on individual parameters",
                ));
            }
            if param_attrs.param_type.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[param(type = ...)]` is only valid on function-level for generic calling convention, not on individual parameters",
                ));
            }
            if param_attrs.if_handle_then_const {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[param(if_handle_then_const)]` is only valid on function-level for generic calling convention, not on individual parameters",
                ));
            }

            return Ok(Some(param_attrs));
        }
    }
    Ok(None)
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
        FunctionKind::TemplateCallback => {
            quote! { Some(::angelscript_core::Behavior::TemplateCallback) }
        }
        FunctionKind::GcGetRefCount => quote! { Some(::angelscript_core::Behavior::GcGetRefCount) },
        FunctionKind::GcSetFlag => quote! { Some(::angelscript_core::Behavior::GcSetFlag) },
        FunctionKind::GcGetFlag => quote! { Some(::angelscript_core::Behavior::GcGetFlag) },
        FunctionKind::GcEnumRefs => quote! { Some(::angelscript_core::Behavior::GcEnumRefs) },
        FunctionKind::GcReleaseRefs => quote! { Some(::angelscript_core::Behavior::GcReleaseRefs) },
        FunctionKind::GetWeakRefFlag => {
            quote! { Some(::angelscript_core::Behavior::GetWeakRefFlag) }
        }
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

    // Check if this is a parameterized variant like "ForValueN(0)"
    if let Some(paren_pos) = variant.find('(') {
        let variant_name = &variant[..paren_pos];
        let args = &variant[paren_pos..]; // includes parentheses, e.g., "(0)"

        let variant_ident = syn::Ident::new(variant_name, proc_macro2::Span::call_site());
        // Parse the argument part as tokens
        let args_tokens: TokenStream2 = args.parse().unwrap_or_else(|_| quote! {});
        quote! { ::angelscript_core::Operator::#variant_ident #args_tokens }
    } else {
        let variant_ident = syn::Ident::new(variant, proc_macro2::Span::call_site());
        quote! { ::angelscript_core::Operator::#variant_ident }
    }
}

/// Generate GenericParamMeta tokens from parsed #[param] attributes.
fn generate_generic_params(param_attrs: &[ParamAttrs]) -> Vec<TokenStream2> {
    param_attrs
        .iter()
        .map(|p| {
            // Priority: template param > param_type (Rust type) > variable > default to VARIABLE_PARAM
            let type_hash = if p.template_param.is_some() {
                // Template parameters use VARIABLE_PARAM - type is resolved at runtime
                quote! { ::angelscript_core::primitives::VARIABLE_PARAM }
            } else if p.is_variable {
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
///
/// For generic functions (`is_generic=true`), the Rust return type is ignored since
/// it's always `Result<(), NativeError>` - the actual return type must be specified
/// via `#[returns]` attribute or defaults to void.
fn generate_return_meta(
    fn_output: &ReturnType,
    return_attrs: &Option<ReturnAttrs>,
    is_generic: bool,
) -> TokenStream2 {
    match return_attrs {
        Some(attrs) => {
            // Get return type - template params use SELF, explicit type, or infer from signature
            let type_hash = if attrs.template_param.is_some() {
                // Template return type - use SELF placeholder, resolved at instantiation
                quote! { Some(::angelscript_core::primitives::SELF) }
            } else if let Some(ty) = &attrs.return_type {
                quote! { Some(<#ty as ::angelscript_core::Any>::type_hash()) }
            } else if is_generic {
                // Generic functions without explicit return type default to void
                quote! { None }
            } else {
                match fn_output {
                    ReturnType::Default => quote! { None },
                    ReturnType::Type(_, ty) => {
                        quote! { Some(<#ty as ::angelscript_core::Any>::type_hash()) }
                    }
                }
            };

            let mode = match attrs.mode {
                ReturnModeAttr::Value => quote! { ::angelscript_core::ReturnMode::Value },
                ReturnModeAttr::Reference => quote! { ::angelscript_core::ReturnMode::Reference },
                ReturnModeAttr::Handle => quote! { ::angelscript_core::ReturnMode::Handle },
            };

            let is_const = attrs.is_const;
            let is_variable = attrs.is_variable;

            // Template parameter name for return type substitution
            let template_param = match &attrs.template_param {
                Some(name) => quote! { Some(#name) },
                None => quote! { None },
            };

            quote! {
                ::angelscript_core::ReturnMeta {
                    type_hash: #type_hash,
                    mode: #mode,
                    is_const: #is_const,
                    is_variable: #is_variable,
                    template_param: #template_param,
                }
            }
        }
        None => {
            // Default: infer from function signature (or void for generic functions)
            if is_generic {
                // Generic functions without #[returns] default to void
                quote! {
                    ::angelscript_core::ReturnMeta {
                        type_hash: None,
                        mode: ::angelscript_core::ReturnMode::Value,
                        is_const: false,
                        is_variable: false,
                        template_param: None,
                    }
                }
            } else {
                match fn_output {
                    ReturnType::Default => quote! {
                        ::angelscript_core::ReturnMeta {
                            type_hash: None,
                            mode: ::angelscript_core::ReturnMode::Value,
                            is_const: false,
                            is_variable: false,
                            template_param: None,
                        }
                    },
                    ReturnType::Type(_, ty) => quote! {
                        ::angelscript_core::ReturnMeta {
                            type_hash: Some(<#ty as ::angelscript_core::Any>::type_hash()),
                            mode: ::angelscript_core::ReturnMode::Value,
                            is_const: false,
                            is_variable: false,
                            template_param: None,
                        }
                    },
                }
            }
        }
    }
}

/// Generate ListPatternMeta token from #[list_pattern] attribute.
fn generate_list_pattern(list_pattern_attrs: &Option<ListPatternAttrs>) -> TokenStream2 {
    match list_pattern_attrs {
        Some(attrs) => match &attrs.pattern {
            // ===== Concrete type variants =====
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
            // ===== Template parameter variants =====
            ListPatternKind::RepeatTemplate(param_name) => {
                // Generate hash using "owner::T" format where owner is Self's type_name
                quote! {
                    Some(::angelscript_core::ListPatternMeta::Repeat(
                        ::angelscript_core::TypeHash::from_name(
                            &format!("{}::{}", <Self as ::angelscript_core::Any>::type_name(), #param_name)
                        )
                    ))
                }
            }
            ListPatternKind::RepeatTupleTemplate(param_names) => {
                // Generate hashes using "owner::K", "owner::V" format
                let hash_tokens: Vec<_> = param_names
                    .iter()
                    .map(|name| {
                        quote! {
                            ::angelscript_core::TypeHash::from_name(
                                &format!("{}::{}", <Self as ::angelscript_core::Any>::type_name(), #name)
                            )
                        }
                    })
                    .collect();
                quote! {
                    Some(::angelscript_core::ListPatternMeta::RepeatTuple(
                        vec![#(#hash_tokens),*]
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
fn filter_param_attrs(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> TokenStream2 {
    let filtered: Vec<_> = inputs
        .iter()
        .map(|arg| {
            match arg {
                FnArg::Receiver(recv) => quote! { #recv },
                FnArg::Typed(pat_type) => {
                    // Filter out #[default], #[template], and #[param] attributes
                    let filtered_attrs: Vec<_> = pat_type
                        .attrs
                        .iter()
                        .filter(|attr| {
                            !attr.path().is_ident("default")
                                && !attr.path().is_ident("template")
                                && !attr.path().is_ident("param")
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
        })
        .collect();

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

/// Generate the NativeFn wrapper for a function.
///
/// For free functions, generates a closure that extracts args from CallContext,
/// calls the mangled function, and sets the return value.
///
/// For methods, handles extracting `this` from slot 0.
///
/// `is_unit_struct` indicates if this is for a standalone function (unit struct pattern)
/// vs a method in an impl block.
fn generate_native_fn(
    fn_name: &str,
    mangled_fn_name: &syn::Ident,
    fn_inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    fn_output: &ReturnType,
    is_generic: bool,
    is_unit_struct: bool,
) -> TokenStream2 {
    // For generic calling convention functions with &mut CallContext signature, box directly
    if is_generic {
        // Check if function takes exactly one &mut CallContext parameter
        let has_call_context_param = fn_inputs.len() == 1
            && fn_inputs.iter().any(|arg| {
                if let FnArg::Typed(pat_type) = arg {
                    let ty_str = quote!(#pat_type.ty).to_string();
                    ty_str.contains("CallContext")
                } else {
                    false
                }
            });

        if has_call_context_param {
            // Box the function directly - it already has the right signature
            // For methods in impl blocks, use Self:: prefix; for unit structs, use bare name
            let fn_ref = if is_unit_struct {
                quote! { #mangled_fn_name }
            } else {
                quote! { Self::#mangled_fn_name }
            };

            return quote! {
                Some(::angelscript_core::NativeFn::new(
                    ::angelscript_core::TypeHash::from_name(#fn_name),
                    #fn_ref
                ))
            };
        }
        // Otherwise, fall through to generate wrapper (for tests/metadata-only generic functions)
    }

    // Extract non-self parameters
    let params: Vec<_> = fn_inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(pat_type) => {
                let name = match pat_type.pat.as_ref() {
                    Pat::Ident(ident) => ident.ident.clone(),
                    _ => return None,
                };
                let ty = pat_type.ty.as_ref().clone();
                Some((name, ty))
            }
        })
        .collect();

    // Check if this is a method with &self or &mut self
    let has_receiver = fn_inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(_)));
    let receiver_is_mut = fn_inputs.iter().any(|arg| {
        if let FnArg::Receiver(r) = arg {
            r.mutability.is_some()
        } else {
            false
        }
    });

    // Check for owned self (fn(self) or fn(mut self)) - not supported in FFI
    for arg in fn_inputs.iter() {
        if let FnArg::Receiver(r) = arg
            && r.reference.is_none()
        {
            let err_msg =
                "owned `self` not supported in FFI methods; use `&self` or `&mut self` instead";
            return quote! {
                compile_error!(#err_msg)
            };
        }
    }

    // Check if any parameter is a non-primitive &T - this creates a borrow conflict with &mut self
    // because both borrow from the same CallContext slots
    let has_ref_param = params.iter().any(|(_, ty)| {
        if let Type::Reference(type_ref) = ty {
            // Check if inner type is a primitive (primitives are copied, not borrowed)
            let inner = &type_ref.elem;
            let inner_str = quote!(#inner).to_string();
            !is_primitive_type(&inner_str)
        } else {
            false
        }
    });

    // For &mut self methods with &T params, we need special handling to avoid borrow conflicts
    // We'll use unsafe pointer access to work around the borrow checker
    let needs_unsafe_self_access = receiver_is_mut && has_ref_param;

    // Generate extraction code for each parameter
    let extractions: Vec<_> = params
        .iter()
        .enumerate()
        .map(|(i, (name, ty))| generate_param_extraction(name, ty, i))
        .collect();

    // Track which params are &mut primitives (need &mut prefix and write-back)
    let mut_primitive_params: Vec<(usize, syn::Ident, Type)> = params
        .iter()
        .enumerate()
        .filter_map(|(i, (name, ty))| {
            if let Type::Reference(type_ref) = ty
                && type_ref.mutability.is_some()
            {
                let base_ty = type_ref.elem.as_ref();
                let base_str = quote!(#base_ty).to_string();
                if is_primitive_type(&base_str) {
                    return Some((i, name.clone(), base_ty.clone()));
                }
            }
            None
        })
        .collect();

    // Generate function call arguments - use &mut for mutable primitive params
    let arg_names: Vec<_> = params
        .iter()
        .map(|(name, ty)| {
            if let Type::Reference(type_ref) = ty
                && type_ref.mutability.is_some()
            {
                let base_ty = type_ref.elem.as_ref();
                let base_str = quote!(#base_ty).to_string();
                if is_primitive_type(&base_str) {
                    // For &mut primitives, we extracted as `let mut name`, so pass &mut name
                    return quote! { &mut #name };
                }
            }
            quote! { #name }
        })
        .collect();

    // Generate write-back code for &mut primitive params
    let writebacks: Vec<_> = mut_primitive_params
        .iter()
        .map(|(i, name, base_ty)| {
            let base_str = quote!(#base_ty).to_string();
            if is_primitive_integer(&base_str) {
                quote! {
                    *__ctx.arg_slot_mut(#i)? = ::angelscript_core::Dynamic::Int(#name as i64);
                }
            } else if is_primitive_float(&base_str) {
                quote! {
                    *__ctx.arg_slot_mut(#i)? = ::angelscript_core::Dynamic::Float(#name as f64);
                }
            } else if base_str == "bool" {
                quote! {
                    *__ctx.arg_slot_mut(#i)? = ::angelscript_core::Dynamic::Bool(#name);
                }
            } else {
                quote! {}
            }
        })
        .collect();

    // Generate return handling - returns None if return type not supported
    let return_handling = match generate_return_handling_code(fn_output) {
        Some(code) => code,
        None => return quote! { None }, // Return None for native_fn if return type not supported
    };

    // Generate the arg_offset for methods (1 for methods with self, 0 for free functions)
    let _arg_offset = if has_receiver { 1usize } else { 0usize };

    // Helper to check if a type is a non-primitive reference
    let is_non_primitive_ref = |ty: &Type| -> bool {
        if let Type::Reference(type_ref) = ty {
            let inner = &type_ref.elem;
            let inner_str = quote!(#inner).to_string();
            !is_primitive_type(&inner_str)
        } else {
            false
        }
    };

    // For &mut self methods with &T params, we need a different code structure
    // to avoid borrow conflicts. We use unsafe pointer access.
    if needs_unsafe_self_access {
        // Find which params are non-primitive &T and need special handling
        let ref_params: Vec<_> = params
            .iter()
            .enumerate()
            .filter(|(_, (_, ty))| is_non_primitive_ref(ty))
            .map(|(i, (name, ty))| {
                let base_ty = if let Type::Reference(type_ref) = ty {
                    type_ref.elem.as_ref().clone()
                } else {
                    ty.clone()
                };
                (i, name.clone(), base_ty)
            })
            .collect();

        // Generate extractions for primitive params only (they don't hold borrows)
        let non_ref_extractions: Vec<_> = params
            .iter()
            .enumerate()
            .filter(|(_, (_, ty))| !is_non_primitive_ref(ty))
            .map(|(i, (name, ty))| generate_param_extraction(name, ty, i))
            .collect();

        // Generate pointer extraction for &T params
        let ref_extractions: Vec<_> = ref_params
            .iter()
            .map(|(i, name, base_ty)| {
                quote! {
                    let #name: *const #base_ty = {
                        let __slot = __ctx.arg_slot(#i)?;
                        match __slot {
                            ::angelscript_core::Dynamic::Native(boxed) => {
                                let __ref = boxed.downcast_ref::<#base_ty>().ok_or_else(|| {
                                    ::angelscript_core::NativeError::other(
                                        concat!("failed to downcast argument ", stringify!(#name))
                                    )
                                })?;
                                __ref as *const #base_ty
                            }
                            _ => return Err(::angelscript_core::NativeError::Conversion(
                                ::angelscript_core::ConversionError::TypeMismatch {
                                    expected: "native",
                                    actual: __slot.type_name(),
                                }
                            )),
                        }
                    };
                }
            })
            .collect();

        // Generate the unsafe dereference for &T params in the call
        let unsafe_derefs: Vec<_> = ref_params
            .iter()
            .map(|(_, name, base_ty)| {
                quote! { let #name: &#base_ty = unsafe { &*#name }; }
            })
            .collect();

        quote! {
            Some(::angelscript_core::NativeFn::new(
                ::angelscript_core::TypeHash::from_name(#fn_name),
                |__ctx: &mut ::angelscript_core::CallContext| {
                    #(#non_ref_extractions)*
                    #(#ref_extractions)*
                    let __result = {
                        let __this: &mut Self = __ctx.this_mut::<Self>()?;
                        #(#unsafe_derefs)*
                        __this.#mangled_fn_name(#(#arg_names),*)
                    };
                    #return_handling
                    Ok(())
                }
            ))
        }
    } else {
        // Normal case - no borrow conflicts
        let call_expr = if has_receiver {
            // Method call: need to extract `this` first
            if receiver_is_mut {
                // Mutable self - use this_mut()
                quote! {
                    {
                        let __this: &mut Self = __ctx.this_mut::<Self>()?;
                        __this.#mangled_fn_name(#(#arg_names),*)
                    }
                }
            } else {
                // Immutable self - use this()
                quote! {
                    {
                        let __this: &Self = __ctx.this::<Self>()?;
                        __this.#mangled_fn_name(#(#arg_names),*)
                    }
                }
            }
        } else if is_unit_struct {
            // True free function (unit struct pattern) - no Self:: prefix
            quote! { #mangled_fn_name(#(#arg_names),*) }
        } else {
            // Associated function call (no self receiver but in impl block)
            // Use Self:: prefix since this is generated inside an impl block
            quote! { Self::#mangled_fn_name(#(#arg_names),*) }
        };

        quote! {
            Some(::angelscript_core::NativeFn::new(
                ::angelscript_core::TypeHash::from_name(#fn_name),
                |__ctx: &mut ::angelscript_core::CallContext| {
                    #(#extractions)*
                    let __result = #call_expr;
                    #(#writebacks)*
                    #return_handling
                    Ok(())
                }
            ))
        }
    }
}

/// Generate code to extract a parameter from CallContext.
fn generate_param_extraction(name: &syn::Ident, ty: &Type, index: usize) -> TokenStream2 {
    // Get the base type (strip references)
    let (base_ty, is_ref, is_mut) = match ty {
        Type::Reference(type_ref) => {
            let is_mut = type_ref.mutability.is_some();
            (type_ref.elem.as_ref(), true, is_mut)
        }
        _ => (ty, false, false),
    };

    // Check if it's a primitive type
    let type_str = quote!(#base_ty).to_string();
    if is_primitive_integer(&type_str) {
        if is_mut {
            // &mut primitive - create local, will write back later
            quote! {
                let mut #name: #base_ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Int(v) => *v as #base_ty,
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "int",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        } else {
            quote! {
                let #name: #ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Int(v) => *v as #base_ty,
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "int",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        }
    } else if is_primitive_float(&type_str) {
        if is_mut {
            // &mut primitive - create local, will write back later
            quote! {
                let mut #name: #base_ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Float(v) => *v as #base_ty,
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "float",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        } else {
            quote! {
                let #name: #ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Float(v) => *v as #base_ty,
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "float",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        }
    } else if type_str == "bool" {
        if is_mut {
            // &mut bool - create local, will write back later
            quote! {
                let mut #name: bool = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Bool(v) => *v,
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "bool",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        } else {
            quote! {
                let #name: #ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Bool(v) => *v,
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "bool",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        }
    } else if type_str == "String" {
        quote! {
            let #name: String = {
                let __slot = __ctx.arg_slot(#index)?;
                match __slot {
                    ::angelscript_core::Dynamic::String(s) => s.clone(),
                    _ => return Err(::angelscript_core::NativeError::Conversion(
                        ::angelscript_core::ConversionError::TypeMismatch {
                            expected: "string",
                            actual: __slot.type_name(),
                        }
                    )),
                }
            };
        }
    } else if type_str == "Dynamic" {
        // Dynamic is already the runtime type - clone it directly from the slot
        quote! {
            let #name: ::angelscript_core::Dynamic = __ctx.arg_slot(#index)?.clone_if_possible()
                .ok_or_else(|| ::angelscript_core::NativeError::other(
                    concat!("cannot clone Dynamic argument ", stringify!(#name))
                ))?;
        }
    } else {
        // Non-primitive type - try to extract from Native or Object
        if is_ref && is_mut {
            // &mut T - mutable borrow from slot
            quote! {
                let #name: &mut #base_ty = {
                    let __slot = __ctx.arg_slot_mut(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Native(boxed) => {
                            boxed.downcast_mut::<#base_ty>().ok_or_else(|| {
                                ::angelscript_core::NativeError::other(
                                    concat!("failed to downcast argument ", stringify!(#name))
                                )
                            })?
                        }
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "native",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        } else if is_ref {
            // &T - immutable borrow from slot
            quote! {
                let #name: &#base_ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Native(boxed) => {
                            boxed.downcast_ref::<#base_ty>().ok_or_else(|| {
                                ::angelscript_core::NativeError::other(
                                    concat!("failed to downcast argument ", stringify!(#name))
                                )
                            })?
                        }
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "native",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        } else {
            // Owned value type - try to downcast and clone from Native
            // This requires the type to implement Clone
            quote! {
                let #name: #ty = {
                    let __slot = __ctx.arg_slot(#index)?;
                    match __slot {
                        ::angelscript_core::Dynamic::Native(boxed) => {
                            boxed.downcast_ref::<#base_ty>().ok_or_else(|| {
                                ::angelscript_core::NativeError::other(
                                    concat!("failed to downcast argument ", stringify!(#name))
                                )
                            })?.clone()
                        }
                        _ => return Err(::angelscript_core::NativeError::Conversion(
                            ::angelscript_core::ConversionError::TypeMismatch {
                                expected: "native",
                                actual: __slot.type_name(),
                            }
                        )),
                    }
                };
            }
        }
    }
}

/// Generate code to handle the return value.
/// Returns Some(token) for supported return types, None for unsupported types.
fn generate_return_handling_code(fn_output: &ReturnType) -> Option<TokenStream2> {
    match fn_output {
        ReturnType::Default => {
            // void return
            Some(quote! {
                let _ = __result;
                __ctx.set_return_slot(::angelscript_core::Dynamic::Void);
            })
        }
        ReturnType::Type(_, ty) => {
            // Check the return type
            let type_str = quote!(#ty).to_string();
            if is_primitive_integer(&type_str) {
                Some(quote! {
                    __ctx.set_return_slot(::angelscript_core::Dynamic::Int(__result as i64));
                })
            } else if is_primitive_float(&type_str) {
                Some(quote! {
                    __ctx.set_return_slot(::angelscript_core::Dynamic::Float(__result as f64));
                })
            } else if type_str == "bool" {
                Some(quote! {
                    __ctx.set_return_slot(::angelscript_core::Dynamic::Bool(__result));
                })
            } else if type_str == "()" {
                Some(quote! {
                    let _ = __result;
                    __ctx.set_return_slot(::angelscript_core::Dynamic::Void);
                })
            } else if type_str == "String" {
                Some(quote! {
                    __ctx.set_return_slot(::angelscript_core::Dynamic::String(__result));
                })
            } else {
                // Non-primitive type - wrap in Dynamic::Native
                Some(quote! {
                    __ctx.set_return_slot(::angelscript_core::Dynamic::Native(
                        Box::new(__result)
                    ));
                })
            }
        }
    }
}
