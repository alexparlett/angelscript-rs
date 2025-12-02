//! Common AST node types used throughout the parser.
//!
//! Provides fundamental types like identifiers, scopes, visibility modifiers,
//! and other shared structures.

use crate::lexer::Span;
use std::fmt;

/// An identifier with source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident<'ast> {
    /// The identifier name (allocated in arena).
    pub name: &'ast str,
    /// Source location.
    pub span: Span,
}

impl<'ast> Ident<'ast> {
    /// Create a new identifier.
    pub fn new(name: &'ast str, span: Span) -> Self {
        Self {
            name,
            span,
        }
    }
}

impl<'ast> fmt::Display for Ident<'ast> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A scope path for namespaces and type references.
///
/// Examples:
/// - `::global` - absolute scope
/// - `Namespace::Type` - relative scope
/// - `::Namespace::SubNamespace::Type` - absolute nested scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scope<'ast> {
    /// Whether this is an absolute scope (starts with `::`)
    pub is_absolute: bool,
    /// The path segments (namespace names)
    pub segments: &'ast [Ident<'ast>],
    /// Source location covering the entire scope
    pub span: Span,
}

impl<'ast> Scope<'ast> {
    /// Create a new scope.
    pub fn new(is_absolute: bool, segments: &'ast [Ident<'ast>], span: Span) -> Self {
        Self {
            is_absolute,
            segments,
            span,
        }
    }

    /// Create an empty scope (no namespace).
    pub fn empty(span: Span) -> Self {
        Self {
            is_absolute: false,
            segments: &[],
            span,
        }
    }

    /// Check if this scope is empty.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl<'ast> fmt::Display for Scope<'ast> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_absolute {
            write!(f, "::")?;
        }
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, "::")?;
            }
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}

/// Visibility modifier for class members and declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Public (default)
    Public,
    /// Private (not accessible outside class)
    Private,
    /// Protected (accessible in derived classes)
    Protected,
}

impl Visibility {
    /// Get the default visibility (public).
    pub fn default() -> Self {
        Self::Public
    }

    /// Check if this is public.
    pub fn is_public(&self) -> bool {
        matches!(self, Self::Public)
    }
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Private => write!(f, "private"),
            Self::Protected => write!(f, "protected"),
        }
    }
}

/// Top-level declaration modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeclModifiers {
    /// `shared` - shared between modules
    pub shared: bool,
    /// `external` - external declaration (implementation elsewhere)
    pub external: bool,
    /// `abstract` - abstract class (cannot be instantiated)
    pub abstract_: bool,
    /// `final` - final class (cannot be inherited from)
    pub final_: bool,
}

impl DeclModifiers {
    /// Create new empty modifiers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any modifiers are set.
    pub fn is_empty(&self) -> bool {
        !self.shared && !self.external && !self.abstract_ && !self.final_
    }
}

impl fmt::Display for DeclModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.shared {
            parts.push("shared");
        }
        if self.external {
            parts.push("external");
        }
        if self.abstract_ {
            parts.push("abstract");
        }
        if self.final_ {
            parts.push("final");
        }
        write!(f, "{}", parts.join(" "))
    }
}

/// Function-specific attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FuncAttr {
    /// `override` - overrides base class method
    pub override_: bool,
    /// `final` - method cannot be overridden
    pub final_: bool,
    /// `explicit` - explicit constructor (no implicit conversions)
    pub explicit: bool,
    /// `property` - property accessor
    pub property: bool,
    /// `delete` - deleted function (cannot be called)
    pub delete: bool,
}

impl FuncAttr {
    /// Create new empty attributes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any attributes are set.
    pub fn is_empty(&self) -> bool {
        !self.override_ && !self.final_ && !self.explicit && !self.property && !self.delete
    }
}

impl fmt::Display for FuncAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.override_ {
            parts.push("override");
        }
        if self.final_ {
            parts.push("final");
        }
        if self.explicit {
            parts.push("explicit");
        }
        if self.property {
            parts.push("property");
        }
        if self.delete {
            parts.push("delete");
        }
        write!(f, "{}", parts.join(" "))
    }
}

/// Reference kind for function parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefKind {
    /// No reference
    None,
    /// `&` reference (mutable by default)
    Ref,
    /// `& in` - input reference (read-only)
    RefIn,
    /// `& out` - output reference (write-only, uninitialized on entry)
    RefOut,
    /// `& inout` - input/output reference (mutable, must be initialized)
    RefInOut,
}

impl RefKind {
    /// Check if this is a reference of any kind.
    pub fn is_ref(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Check if this reference allows reading.
    pub fn is_readable(&self) -> bool {
        matches!(self, Self::Ref | Self::RefIn | Self::RefInOut)
    }

    /// Check if this reference allows writing.
    pub fn is_writable(&self) -> bool {
        matches!(self, Self::Ref | Self::RefOut | Self::RefInOut)
    }
}

impl fmt::Display for RefKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, ""),
            Self::Ref => write!(f, "&"),
            Self::RefIn => write!(f, "& in"),
            Self::RefOut => write!(f, "& out"),
            Self::RefInOut => write!(f, "& inout"),
        }
    }
}

/// Virtual property accessor kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyAccessorKind {
    /// Getter (read access)
    Get,
    /// Setter (write access)
    Set,
}

impl fmt::Display for PropertyAccessorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => write!(f, "get"),
            Self::Set => write!(f, "set"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ident_display() {
        let ident = Ident::new("myVar", Span::new(1, 1, 5));
        assert_eq!(format!("{}", ident), "myVar");
    }

    #[test]
    fn scope_display() {
        let arena = bumpalo::Bump::new();
        let segments = bumpalo::vec![in &arena;
            Ident::new("Namespace", Span::new(1, 1, 9)),
            Ident::new("Type", Span::new(1, 12, 4)),
        ];
        let scope = Scope::new(false, segments.into_bump_slice(), Span::new(1, 1, 15));
        assert_eq!(format!("{}", scope), "Namespace::Type");

        let absolute = Scope::new(true, &[], Span::new(1, 1, 2));
        assert_eq!(format!("{}", absolute), "::");
    }

    #[test]
    fn visibility_default() {
        let vis = Visibility::default();
        assert!(vis.is_public());
    }

    #[test]
    fn modifiers_empty() {
        let mods = DeclModifiers::new();
        assert!(mods.is_empty());

        let mods = DeclModifiers {
            shared: true,
            ..Default::default()
        };
        assert!(!mods.is_empty());
    }

    #[test]
    fn ref_kind_checks() {
        assert!(!RefKind::None.is_ref());
        assert!(RefKind::Ref.is_ref());
        assert!(RefKind::RefIn.is_readable());
        assert!(!RefKind::RefIn.is_writable());
        assert!(RefKind::RefOut.is_writable());
        assert!(!RefKind::RefOut.is_readable());
    }

    #[test]
    fn all_ref_kind_variants() {
        // Test all RefKind methods comprehensively
        assert!(!RefKind::None.is_ref());
        assert!(!RefKind::None.is_readable());
        assert!(!RefKind::None.is_writable());

        assert!(RefKind::Ref.is_ref());
        assert!(RefKind::Ref.is_readable());
        assert!(RefKind::Ref.is_writable());

        assert!(RefKind::RefIn.is_ref());
        assert!(RefKind::RefIn.is_readable());
        assert!(!RefKind::RefIn.is_writable());

        assert!(RefKind::RefOut.is_ref());
        assert!(!RefKind::RefOut.is_readable());
        assert!(RefKind::RefOut.is_writable());

        assert!(RefKind::RefInOut.is_ref());
        assert!(RefKind::RefInOut.is_readable());
        assert!(RefKind::RefInOut.is_writable());
    }

    #[test]
    fn all_ref_kind_display() {
        assert_eq!(format!("{}", RefKind::None), "");
        assert_eq!(format!("{}", RefKind::Ref), "&");
        assert_eq!(format!("{}", RefKind::RefIn), "& in");
        assert_eq!(format!("{}", RefKind::RefOut), "& out");
        assert_eq!(format!("{}", RefKind::RefInOut), "& inout");
    }

    #[test]
    fn all_visibility_display() {
        assert_eq!(format!("{}", Visibility::Public), "public");
        assert_eq!(format!("{}", Visibility::Private), "private");
        assert_eq!(format!("{}", Visibility::Protected), "protected");
    }

    #[test]
    fn visibility_is_public() {
        assert!(Visibility::Public.is_public());
        assert!(!Visibility::Private.is_public());
        assert!(!Visibility::Protected.is_public());
    }

    #[test]
    fn decl_modifiers_display() {
        let mut mods = DeclModifiers::new();
        assert_eq!(format!("{}", mods), "");

        mods.shared = true;
        assert_eq!(format!("{}", mods), "shared");

        mods.external = true;
        assert!(format!("{}", mods).contains("shared"));
        assert!(format!("{}", mods).contains("external"));

        mods.abstract_ = true;
        mods.final_ = true;
        let display = format!("{}", mods);
        assert!(display.contains("shared"));
        assert!(display.contains("external"));
        assert!(display.contains("abstract"));
        assert!(display.contains("final"));
    }

    #[test]
    fn decl_modifiers_is_empty() {
        let mut mods = DeclModifiers::new();
        assert!(mods.is_empty());

        mods.shared = true;
        assert!(!mods.is_empty());

        mods = DeclModifiers::new();
        mods.external = true;
        assert!(!mods.is_empty());

        mods = DeclModifiers::new();
        mods.abstract_ = true;
        assert!(!mods.is_empty());

        mods = DeclModifiers::new();
        mods.final_ = true;
        assert!(!mods.is_empty());
    }

    #[test]
    fn func_attr_display() {
        let mut attr = FuncAttr::new();
        assert_eq!(format!("{}", attr), "");

        attr.override_ = true;
        assert_eq!(format!("{}", attr), "override");

        attr.final_ = true;
        assert!(format!("{}", attr).contains("override"));
        assert!(format!("{}", attr).contains("final"));

        attr.explicit = true;
        attr.property = true;
        attr.delete = true;
        let display = format!("{}", attr);
        assert!(display.contains("override"));
        assert!(display.contains("final"));
        assert!(display.contains("explicit"));
        assert!(display.contains("property"));
        assert!(display.contains("delete"));
    }

    #[test]
    fn func_attr_is_empty() {
        let mut attr = FuncAttr::new();
        assert!(attr.is_empty());

        attr.override_ = true;
        assert!(!attr.is_empty());

        attr = FuncAttr::new();
        attr.final_ = true;
        assert!(!attr.is_empty());

        attr = FuncAttr::new();
        attr.explicit = true;
        assert!(!attr.is_empty());

        attr = FuncAttr::new();
        attr.property = true;
        assert!(!attr.is_empty());

        attr = FuncAttr::new();
        attr.delete = true;
        assert!(!attr.is_empty());
    }

    #[test]
    fn property_accessor_kind_display() {
        assert_eq!(format!("{}", PropertyAccessorKind::Get), "get");
        assert_eq!(format!("{}", PropertyAccessorKind::Set), "set");
    }

    #[test]
    fn scope_empty() {
        let scope = Scope::empty(Span::new(1, 1, 0));
        assert!(scope.is_empty());
        assert!(!scope.is_absolute);
        assert_eq!(scope.segments.len(), 0);
    }

    #[test]
    fn scope_is_empty_with_segments() {
        let arena = bumpalo::Bump::new();
        let segments = bumpalo::vec![in &arena; Ident::new("Test", Span::new(1, 1, 4))];
        let scope = Scope::new(false, segments.into_bump_slice(), Span::new(1, 1, 4));
        assert!(!scope.is_empty());
    }

    #[test]
    fn scope_display_absolute() {
        let arena = bumpalo::Bump::new();
        let segments = bumpalo::vec![in &arena;
            Ident::new("A", Span::new(1, 3, 1)),
            Ident::new("B", Span::new(1, 7, 1)),
        ];
        let scope = Scope::new(true, segments.into_bump_slice(), Span::new(1, 1, 9));
        assert_eq!(format!("{}", scope), "::A::B");
    }

    #[test]
    fn scope_display_empty() {
        let scope = Scope::empty(Span::new(1, 1, 0));
        assert_eq!(format!("{}", scope), "");
    }
}
