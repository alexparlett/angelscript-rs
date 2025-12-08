//! Attribute parsing utilities for AngelScript macros.

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Expr, Ident, LitStr, Token,
};

/// Parsed `#[angelscript(...)]` attributes on a type.
#[derive(Debug, Default)]
pub struct TypeAttrs {
    /// Override name for AngelScript (default: Rust struct name)
    pub name: Option<String>,
    /// Type kind modifier
    pub type_kind: Option<TypeKindAttr>,
    /// Template parameter string (e.g., "<T>" or "<K, V>")
    pub template: Option<String>,
    /// Template specialization: base template name.
    /// Example: `specialization_of = "myTemplate"`
    pub specialization_of: Option<String>,
    /// Template specialization arguments as types.
    /// Example: `specialization_args(f32, i32)`
    pub specialization_args: Vec<syn::Type>,
}

/// Type kind attribute values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKindAttr {
    Value,
    Pod,
    Reference,
    Scoped,
    NoCount,
    AsHandle,
}

/// Parsed `#[angelscript(...)]` attributes on a field.
#[derive(Debug, Default)]
pub struct FieldAttrs {
    /// Generate getter
    pub get: bool,
    /// Generate setter
    pub set: bool,
    /// Override property name
    pub name: Option<String>,
}

/// Parsed `#[angelscript::function(...)]` attributes.
#[derive(Debug, Default)]
pub struct FunctionAttrs {
    /// Override name for AngelScript
    pub name: Option<String>,
    /// Function kind
    pub kind: FunctionKind,
    /// Is const method
    pub is_const: bool,
    /// Is property accessor
    pub is_property: bool,
    /// Property name override (overrides inference from get_/set_ prefix)
    pub property_name: Option<String>,
    /// Is generic calling convention
    pub is_generic: bool,
    /// Is template function (deprecated, use template = "...")
    pub is_template: bool,
    /// Template parameter string for template functions (e.g., "<T, U>").
    /// Example: `template = "<T, U>"` for `T Test<T, U>(T t, U u)`
    pub template: Option<String>,
    /// Operator type
    pub operator: Option<String>,
    /// Explicit return type for generic functions
    pub returns: Option<String>,
    /// Copy constructor
    pub is_copy: bool,
    /// Keep original function name callable (use __meta suffix for metadata)
    pub keep: bool,
}

/// Function kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FunctionKind {
    #[default]
    Global,
    Instance,
    Constructor,
    Factory,
    Destructor,
    AddRef,
    Release,
    ListConstruct,
    ListFactory,
    TemplateCallback,
    // GC behaviors
    GcGetRefCount,
    GcSetFlag,
    GcGetFlag,
    GcEnumRefs,
    GcReleaseRefs,
    GetWeakRefFlag,
}

impl TypeAttrs {
    /// Parse attributes from a list of `#[angelscript(...)]` attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("angelscript") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value: LitStr = meta.value()?.parse()?;
                    result.name = Some(value.value());
                } else if meta.path.is_ident("value") {
                    result.type_kind = Some(TypeKindAttr::Value);
                } else if meta.path.is_ident("pod") {
                    result.type_kind = Some(TypeKindAttr::Pod);
                } else if meta.path.is_ident("reference") {
                    result.type_kind = Some(TypeKindAttr::Reference);
                } else if meta.path.is_ident("scoped") {
                    result.type_kind = Some(TypeKindAttr::Scoped);
                } else if meta.path.is_ident("nocount") {
                    result.type_kind = Some(TypeKindAttr::NoCount);
                } else if meta.path.is_ident("as_handle") {
                    result.type_kind = Some(TypeKindAttr::AsHandle);
                } else if meta.path.is_ident("template") {
                    let value: LitStr = meta.value()?.parse()?;
                    result.template = Some(value.value());
                } else if meta.path.is_ident("specialization_of") {
                    let value: LitStr = meta.value()?.parse()?;
                    result.specialization_of = Some(value.value());
                } else if meta.path.is_ident("specialization_args") {
                    // Parse parenthesized list of types: specialization_args(f32, i32)
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let types: Punctuated<syn::Type, Token![,]> =
                        content.parse_terminated(syn::Type::parse, Token![,])?;
                    result.specialization_args = types.into_iter().collect();
                } else {
                    return Err(meta.error(format!(
                        "unknown angelscript attribute: {}",
                        meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

impl FieldAttrs {
    /// Parse attributes from a list of `#[angelscript(...)]` attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("angelscript") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("get") {
                    result.get = true;
                } else if meta.path.is_ident("set") {
                    result.set = true;
                } else if meta.path.is_ident("name") {
                    let value: LitStr = meta.value()?.parse()?;
                    result.name = Some(value.value());
                } else {
                    return Err(meta.error(format!(
                        "unknown angelscript field attribute: {}",
                        meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                    )));
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

impl FunctionAttrs {
    /// Parse function attributes from the attribute token stream.
    pub fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();

        if input.is_empty() {
            return Ok(result);
        }

        let items = Punctuated::<FunctionAttrItem, Token![,]>::parse_terminated(input)?;

        for item in items {
            match item {
                FunctionAttrItem::Const => {
                    result.is_const = true;
                }
                FunctionAttrItem::Ident(ident) => {
                    let name = ident.to_string();
                    match name.as_str() {
                        "instance" => result.kind = FunctionKind::Instance,
                        "constructor" => result.kind = FunctionKind::Constructor,
                        "factory" => result.kind = FunctionKind::Factory,
                        "destructor" => result.kind = FunctionKind::Destructor,
                        "addref" => result.kind = FunctionKind::AddRef,
                        "release" => result.kind = FunctionKind::Release,
                        "list_construct" => result.kind = FunctionKind::ListConstruct,
                        "list_factory" => result.kind = FunctionKind::ListFactory,
                        "template_callback" => result.kind = FunctionKind::TemplateCallback,
                        "gc_getrefcount" => result.kind = FunctionKind::GcGetRefCount,
                        "gc_setflag" => result.kind = FunctionKind::GcSetFlag,
                        "gc_getflag" => result.kind = FunctionKind::GcGetFlag,
                        "gc_enumrefs" => result.kind = FunctionKind::GcEnumRefs,
                        "gc_releaserefs" => result.kind = FunctionKind::GcReleaseRefs,
                        "get_weakref_flag" => result.kind = FunctionKind::GetWeakRefFlag,
                        "const" => result.is_const = true,
                        "property" => result.is_property = true,
                        "generic" => result.is_generic = true,
                        "template" => result.is_template = true,
                        "copy" => result.is_copy = true,
                        "keep" => result.keep = true,
                        _ => {
                            return Err(syn::Error::new(
                                ident.span(),
                                format!("unknown function attribute: {}", name),
                            ))
                        }
                    }
                }
                FunctionAttrItem::NameValue { name, value } => {
                    let name_str = name.to_string();
                    match name_str.as_str() {
                        "name" => {
                            if let Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = value {
                                result.name = Some(s.value());
                            } else {
                                return Err(syn::Error::new(
                                    name.span(),
                                    "name value must be a string literal",
                                ));
                            }
                        }
                        "property_name" => {
                            if let Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = value {
                                result.property_name = Some(s.value());
                            } else {
                                return Err(syn::Error::new(
                                    name.span(),
                                    "property_name value must be a string literal",
                                ));
                            }
                        }
                        "operator" => {
                            if let Expr::Path(path) = value {
                                result.operator = Some(
                                    path.path
                                        .segments
                                        .iter()
                                        .map(|s| s.ident.to_string())
                                        .collect::<Vec<_>>()
                                        .join("::"),
                                );
                            } else {
                                return Err(syn::Error::new(
                                    name.span(),
                                    "operator value must be a path like Operator::Add",
                                ));
                            }
                        }
                        "returns" => {
                            if let Expr::Path(path) = value {
                                result.returns = Some(
                                    path.path
                                        .segments
                                        .iter()
                                        .map(|s| s.ident.to_string())
                                        .collect::<Vec<_>>()
                                        .join("::"),
                                );
                            } else {
                                return Err(syn::Error::new(
                                    name.span(),
                                    "returns value must be a type path",
                                ));
                            }
                        }
                        "template" => {
                            if let Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = value {
                                result.template = Some(s.value());
                            } else {
                                return Err(syn::Error::new(
                                    name.span(),
                                    "template value must be a string literal like \"<T, U>\"",
                                ));
                            }
                        }
                        _ => {
                            return Err(syn::Error::new(
                                name.span(),
                                format!("unknown function attribute: {}", name_str),
                            ))
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Individual function attribute item.
enum FunctionAttrItem {
    /// Simple identifier (e.g., `instance`)
    Ident(Ident),
    /// The `const` keyword (handled specially as it's a reserved word)
    Const,
    /// Name = value (e.g., `operator = Operator::Add`)
    NameValue { name: Ident, value: Expr },
}

// =============================================================================
// Param Attribute
// =============================================================================

/// Reference mode for parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefModeAttr {
    /// No reference modifier (by value).
    #[default]
    None,
    /// Input reference (`in`).
    In,
    /// Output reference (`out`).
    Out,
    /// Input/output reference (`inout`).
    InOut,
}

/// Parsed `#[param(...)]` attribute for generic calling convention parameters.
///
/// Supports:
/// - `variable` - Any type parameter (`?`)
/// - `variadic` - Multiple parameters of same type (`...`)
/// - `type = T` - Specific type
/// - `in` / `out` / `inout` - Reference mode
/// - `default = "expr"` - Default value expression
/// - `if_handle_then_const` - When T is handle type, pointed-to object is also const
#[derive(Debug, Default)]
pub struct ParamAttrs {
    /// Is this a variable type (`?`)?
    pub is_variable: bool,
    /// Is this variadic (`...`)?
    pub is_variadic: bool,
    /// Explicit type (None means infer or variable)
    pub param_type: Option<syn::Type>,
    /// Reference mode
    pub ref_mode: RefModeAttr,
    /// Default value expression (e.g., "-1", "\"\"")
    pub default: Option<String>,
    /// If true and this is a template param instantiated with a handle type,
    /// the pointed-to object is also const.
    pub if_handle_then_const: bool,
}

impl ParamAttrs {
    /// Parse a single `#[param(...)]` attribute.
    pub fn from_attr(attr: &Attribute) -> syn::Result<Self> {
        let mut result = Self::default();

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("variable") {
                result.is_variable = true;
            } else if meta.path.is_ident("variadic") {
                result.is_variadic = true;
            } else if meta.path.is_ident("in") {
                result.ref_mode = RefModeAttr::In;
            } else if meta.path.is_ident("out") {
                result.ref_mode = RefModeAttr::Out;
            } else if meta.path.is_ident("inout") {
                result.ref_mode = RefModeAttr::InOut;
            } else if meta.path.is_ident("type") {
                let value = meta.value()?;
                let ty: syn::Type = value.parse()?;
                result.param_type = Some(ty);
            } else if meta.path.is_ident("default") {
                let value = meta.value()?;
                let lit: LitStr = value.parse()?;
                result.default = Some(lit.value());
            } else if meta.path.is_ident("if_handle_then_const") {
                result.if_handle_then_const = true;
            } else {
                return Err(meta.error(format!(
                    "unknown param attribute: {}",
                    meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                )));
            }
            Ok(())
        })?;

        Ok(result)
    }

    /// Parse all `#[param(...)]` attributes from a list.
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Vec<Self>> {
        let mut params = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("param") {
                params.push(Self::from_attr(attr)?);
            }
        }

        Ok(params)
    }
}

// =============================================================================
// Return Attribute
// =============================================================================

/// Return mode for functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReturnModeAttr {
    /// Return by value (default).
    #[default]
    Value,
    /// Return by reference.
    Reference,
    /// Return as handle.
    Handle,
}

/// Parsed `#[return(...)]` attribute for return type metadata.
///
/// Supports:
/// - `const` - Const reference/handle return
/// - `ref` - Return by reference
/// - `handle` - Return as handle
/// - `variable` - Variable type return (`?`)
/// - `type = T` - Explicit return type (for generic calling conv)
#[derive(Debug, Default)]
pub struct ReturnAttrs {
    /// Return mode (value, reference, handle)
    pub mode: ReturnModeAttr,
    /// Is the return const?
    pub is_const: bool,
    /// Is this a variable type return?
    pub is_variable: bool,
    /// Explicit return type (for generic calling convention)
    pub return_type: Option<syn::Type>,
}

impl ReturnAttrs {
    /// Parse a `#[return(...)]` attribute.
    pub fn from_attr(attr: &Attribute) -> syn::Result<Self> {
        let mut result = Self::default();

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("const") {
                result.is_const = true;
            } else if meta.path.is_ident("ref") {
                result.mode = ReturnModeAttr::Reference;
            } else if meta.path.is_ident("handle") {
                result.mode = ReturnModeAttr::Handle;
            } else if meta.path.is_ident("variable") {
                result.is_variable = true;
            } else if meta.path.is_ident("type") {
                let value = meta.value()?;
                let ty: syn::Type = value.parse()?;
                result.return_type = Some(ty);
            } else {
                return Err(meta.error(format!(
                    "unknown return attribute: {}",
                    meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                )));
            }
            Ok(())
        })?;

        Ok(result)
    }

    /// Find and parse the first `#[return(...)]` attribute from a list.
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("return") {
                // Note: `return` is a keyword, so we need special handling
                // Actually, let's use `returns` to avoid keyword conflict
                return Ok(Some(Self::from_attr(attr)?));
            }
            if attr.path().is_ident("returns") {
                return Ok(Some(Self::from_attr(attr)?));
            }
        }
        Ok(None)
    }
}

// =============================================================================
// List Pattern Attribute
// =============================================================================

/// List pattern kind for initialization lists.
#[derive(Debug, Clone)]
pub enum ListPatternKind {
    /// Repeat a single type: `{T, T, T, ...}`
    Repeat(syn::Type),
    /// Fixed sequence of types: `{A, B, C}`
    Fixed(Vec<syn::Type>),
    /// Repeat a tuple of types: `{(K, V), (K, V), ...}`
    RepeatTuple(Vec<syn::Type>),
}

/// Parsed `#[list_pattern(...)]` attribute for list constructors/factories.
///
/// Supports:
/// - `repeat = T` - Repeating single type
/// - `fixed(T1, T2, T3)` - Fixed sequence
/// - `repeat_tuple(K, V)` - Repeating tuple pattern
#[derive(Debug)]
pub struct ListPatternAttrs {
    /// The list pattern kind
    pub pattern: ListPatternKind,
}

impl ListPatternAttrs {
    /// Parse a `#[list_pattern(...)]` attribute.
    pub fn from_attr(attr: &Attribute) -> syn::Result<Self> {
        let mut pattern: Option<ListPatternKind> = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("repeat") {
                let value = meta.value()?;
                let ty: syn::Type = value.parse()?;
                pattern = Some(ListPatternKind::Repeat(ty));
            } else if meta.path.is_ident("fixed") {
                // Parse parenthesized list of types
                let content;
                syn::parenthesized!(content in meta.input);
                let types: Punctuated<syn::Type, Token![,]> =
                    content.parse_terminated(syn::Type::parse, Token![,])?;
                pattern = Some(ListPatternKind::Fixed(types.into_iter().collect()));
            } else if meta.path.is_ident("repeat_tuple") {
                // Parse parenthesized list of types for tuple
                let content;
                syn::parenthesized!(content in meta.input);
                let types: Punctuated<syn::Type, Token![,]> =
                    content.parse_terminated(syn::Type::parse, Token![,])?;
                pattern = Some(ListPatternKind::RepeatTuple(types.into_iter().collect()));
            } else {
                return Err(meta.error(format!(
                    "unknown list_pattern attribute: {}",
                    meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default()
                )));
            }
            Ok(())
        })?;

        pattern
            .map(|p| ListPatternAttrs { pattern: p })
            .ok_or_else(|| syn::Error::new_spanned(attr, "list_pattern requires a pattern specification"))
    }

    /// Find and parse the first `#[list_pattern(...)]` attribute from a list.
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Option<Self>> {
        for attr in attrs {
            if attr.path().is_ident("list_pattern") {
                return Ok(Some(Self::from_attr(attr)?));
            }
        }
        Ok(None)
    }
}

impl Parse for FunctionAttrItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Handle `const` keyword specially
        if input.peek(Token![const]) {
            let _: Token![const] = input.parse()?;
            return Ok(FunctionAttrItem::Const);
        }

        let ident: Ident = input.parse()?;

        if input.peek(Token![=]) {
            let _: Token![=] = input.parse()?;
            let value: Expr = input.parse()?;
            Ok(FunctionAttrItem::NameValue { name: ident, value })
        } else {
            Ok(FunctionAttrItem::Ident(ident))
        }
    }
}

