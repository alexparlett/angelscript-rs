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
    /// Function kind
    pub kind: FunctionKind,
    /// Is const method
    pub is_const: bool,
    /// Is property accessor
    pub is_property: bool,
    /// Is generic calling convention
    pub is_generic: bool,
    /// Is template function
    pub is_template: bool,
    /// Operator type
    pub operator: Option<String>,
    /// Explicit return type for generic functions
    pub returns: Option<String>,
    /// Copy constructor
    pub is_copy: bool,
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
    /// Simple identifier (e.g., `instance`, `const`)
    Ident(Ident),
    /// Name = value (e.g., `operator = Operator::Add`)
    NameValue { name: Ident, value: Expr },
}

impl Parse for FunctionAttrItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
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
