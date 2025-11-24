//! Type expression AST nodes for AngelScript.
//!
//! Provides nodes for representing all type expressions including:
//! - Primitive types (int, float, void, etc.)
//! - User-defined types (MyClass, array<T>)
//! - Scoped types (Namespace::Type)
//! - Template types (array<int>, dict<string, int>)
//! - Type modifiers (const, references, handles)
//! - Array and handle suffixes ([], @, @ const)
//!
//! # Example Type Expressions
//!
//! ```text
//! int                              // Primitive
//! const int                        // Const primitive
//! MyClass                          // User type
//! Namespace::MyClass               // Scoped type
//! array<int>                       // Template type
//! const array<int>[]               // Const template array
//! MyClass@                         // Handle
//! const MyClass@                   // Handle to const object
//! MyClass@ const                   // Const handle
//! const MyClass@ const             // Const handle to const object
//! Namespace::Type<T>[]@            // Complex: scoped template array handle
//! ```

use crate::ast::{Ident, Scope};
use crate::lexer::Span;
use std::fmt;

/// A complete type expression.
///
/// Examples:
/// - `int` - simple type
/// - `const array<int>[]` - const template with array suffix
/// - `MyClass@ const` - const handle
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    /// Leading const (makes the object const, not the handle)
    pub is_const: bool,
    /// Optional namespace scope
    pub scope: Option<Scope>,
    /// The base type
    pub base: TypeBase,
    /// Template arguments if this is a template type
    pub template_args: Vec<TypeExpr>,
    /// Type suffixes (arrays, handles)
    pub suffixes: Vec<TypeSuffix>,
    /// Source location
    pub span: Span,
}

impl TypeExpr {
    /// Create a new type expression.
    pub fn new(
        is_const: bool,
        scope: Option<Scope>,
        base: TypeBase,
        template_args: Vec<TypeExpr>,
        suffixes: Vec<TypeSuffix>,
        span: Span,
    ) -> Self {
        Self {
            is_const,
            scope,
            base,
            template_args,
            suffixes,
            span,
        }
    }

    /// Create a simple primitive type.
    pub fn primitive(prim: PrimitiveType, span: Span) -> Self {
        Self {
            is_const: false,
            scope: None,
            base: TypeBase::Primitive(prim),
            template_args: Vec::new(),
            suffixes: Vec::new(),
            span,
        }
    }

    /// Create a simple named type.
    pub fn named(name: Ident) -> Self {
        let span = name.span;
        Self {
            is_const: false,
            scope: None,
            base: TypeBase::Named(name),
            template_args: Vec::new(),
            suffixes: Vec::new(),
            span,
        }
    }

    /// Check if this type has any handles (@).
    pub fn has_handle(&self) -> bool {
        self.suffixes.iter().any(|s| matches!(s, TypeSuffix::Handle { .. }))
    }

    /// Check if this type has any arrays ([]).
    pub fn has_array(&self) -> bool {
        self.suffixes.iter().any(|s| matches!(s, TypeSuffix::Array))
    }

    /// Check if this type is a reference type (has @ handle).
    pub fn is_reference_type(&self) -> bool {
        self.has_handle()
    }

    /// Check if this is a void type.
    pub fn is_void(&self) -> bool {
        matches!(self.base, TypeBase::Primitive(PrimitiveType::Void))
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_const {
            write!(f, "const ")?;
        }
        if let Some(scope) = &self.scope {
            write!(f, "{}::", scope)?;
        }
        write!(f, "{}", self.base)?;
        if !self.template_args.is_empty() {
            write!(f, "<")?;
            for (i, arg) in self.template_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ">")?;
        }
        for suffix in &self.suffixes {
            write!(f, "{}", suffix)?;
        }
        Ok(())
    }
}

/// The base type without modifiers or suffixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeBase {
    /// Primitive type (int, float, void, etc.)
    Primitive(PrimitiveType),
    /// Named user-defined type or identifier
    Named(Ident),
    /// Auto type (compiler infers)
    Auto,
    /// Unknown/placeholder type (?)
    Unknown,
}

impl fmt::Display for TypeBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(p) => write!(f, "{}", p),
            Self::Named(name) => write!(f, "{}", name),
            Self::Auto => write!(f, "auto"),
            Self::Unknown => write!(f, "?"),
        }
    }
}

/// Primitive types in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    /// void
    Void,
    /// bool
    Bool,
    /// int (int32)
    Int,
    /// int8
    Int8,
    /// int16
    Int16,
    /// int64
    Int64,
    /// uint (uint32)
    UInt,
    /// uint8
    UInt8,
    /// uint16
    UInt16,
    /// uint64
    UInt64,
    /// float
    Float,
    /// double
    Double,
}

impl PrimitiveType {
    /// Get the size of this primitive type in bytes.
    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Void => 0,
            Self::Bool | Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int | Self::UInt | Self::Float => 4,
            Self::Int64 | Self::UInt64 | Self::Double => 8,
        }
    }

    /// Check if this is an integer type.
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int | Self::Int8 | Self::Int16 | Self::Int64 |
            Self::UInt | Self::UInt8 | Self::UInt16 | Self::UInt64
        )
    }

    /// Check if this is a floating-point type.
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float | Self::Double)
    }

    /// Check if this is a signed integer type.
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            Self::Int | Self::Int8 | Self::Int16 | Self::Int64
        )
    }

    /// Check if this is an unsigned integer type.
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Self::UInt | Self::UInt8 | Self::UInt16 | Self::UInt64
        )
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Void => "void",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Int8 => "int8",
            Self::Int16 => "int16",
            Self::Int64 => "int64",
            Self::UInt => "uint",
            Self::UInt8 => "uint8",
            Self::UInt16 => "uint16",
            Self::UInt64 => "uint64",
            Self::Float => "float",
            Self::Double => "double",
        };
        write!(f, "{}", s)
    }
}

/// Type suffixes that modify the base type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSuffix {
    /// Array suffix: `[]`
    Array,
    /// Handle suffix: `@` with optional trailing const
    /// 
    /// Examples:
    /// - `MyClass@` - handle (is_const = false)
    /// - `MyClass@ const` - const handle (is_const = true)
    Handle {
        /// Whether the handle itself is const (trailing const)
        is_const: bool,
    },
}

impl fmt::Display for TypeSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Array => write!(f, "[]"),
            Self::Handle { is_const: false } => write!(f, "@"),
            Self::Handle { is_const: true } => write!(f, "@ const"),
        }
    }
}

/// A return type for functions, which can include reference modifiers.
///
/// Examples:
/// - `void` - simple return
/// - `int&` - return by reference
/// - `const string&` - return const reference
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnType {
    /// The base type
    pub ty: TypeExpr,
    /// Whether this is returned by reference (&)
    pub is_ref: bool,
    /// Source location
    pub span: Span,
}

impl ReturnType {
    /// Create a new return type.
    pub fn new(ty: TypeExpr, is_ref: bool, span: Span) -> Self {
        Self { ty, is_ref, span }
    }
}

impl fmt::Display for ReturnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ty)?;
        if self.is_ref {
            write!(f, "&")?;
        }
        Ok(())
    }
}

/// Parameter type with modifiers.
///
/// Examples:
/// - `int` - by value
/// - `int&` - by reference (mutable)
/// - `const int&` - by const reference
/// - `int& in` - input reference
/// - `int& out` - output reference
/// - `int& inout` - input/output reference
#[derive(Debug, Clone, PartialEq)]
pub struct ParamType {
    /// The base type
    pub ty: TypeExpr,
    /// Reference kind (None, Ref, RefIn, RefOut, RefInOut)
    pub ref_kind: crate::ast::RefKind,
    /// Source location
    pub span: Span,
}

impl ParamType {
    /// Create a new parameter type.
    pub fn new(ty: TypeExpr, ref_kind: crate::ast::RefKind, span: Span) -> Self {
        Self { ty, ref_kind, span }
    }

    /// Check if this parameter is by reference.
    pub fn is_ref(&self) -> bool {
        self.ref_kind.is_ref()
    }
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.ty)?;
        if self.ref_kind.is_ref() {
            write!(f, " {}", self.ref_kind)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type_sizes() {
        assert_eq!(PrimitiveType::Void.size_bytes(), 0);
        assert_eq!(PrimitiveType::Bool.size_bytes(), 1);
        assert_eq!(PrimitiveType::Int8.size_bytes(), 1);
        assert_eq!(PrimitiveType::Int16.size_bytes(), 2);
        assert_eq!(PrimitiveType::Int.size_bytes(), 4);
        assert_eq!(PrimitiveType::Int64.size_bytes(), 8);
        assert_eq!(PrimitiveType::Float.size_bytes(), 4);
        assert_eq!(PrimitiveType::Double.size_bytes(), 8);
    }

    #[test]
    fn primitive_type_checks() {
        assert!(PrimitiveType::Int.is_integer());
        assert!(PrimitiveType::Int.is_signed());
        assert!(!PrimitiveType::Int.is_unsigned());
        
        assert!(PrimitiveType::UInt.is_integer());
        assert!(PrimitiveType::UInt.is_unsigned());
        assert!(!PrimitiveType::UInt.is_signed());
        
        assert!(PrimitiveType::Float.is_float());
        assert!(!PrimitiveType::Float.is_integer());
    }

    #[test]
    fn simple_type_display() {
        let ty = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 0 + 1, 3 - 0));
        assert_eq!(format!("{}", ty), "int");
    }

    #[test]
    fn const_type_display() {
        let mut ty = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 0 + 1, 9 - 0));
        ty.is_const = true;
        assert_eq!(format!("{}", ty), "const int");
    }

    #[test]
    fn handle_type_display() {
        let mut ty = TypeExpr::named(Ident::new("MyClass", Span::new(1, 0 + 1, 7 - 0)));
        ty.suffixes.push(TypeSuffix::Handle { is_const: false });
        assert_eq!(format!("{}", ty), "MyClass@");
    }

    #[test]
    fn const_handle_display() {
        let mut ty = TypeExpr::named(Ident::new("MyClass", Span::new(1, 0 + 1, 7 - 0)));
        ty.suffixes.push(TypeSuffix::Handle { is_const: true });
        assert_eq!(format!("{}", ty), "MyClass@ const");
    }

    #[test]
    fn array_type_display() {
        let mut ty = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 0 + 1, 5 - 0));
        ty.suffixes.push(TypeSuffix::Array);
        assert_eq!(format!("{}", ty), "int[]");
    }

    #[test]
    fn template_type_display() {
        let mut ty = TypeExpr::named(Ident::new("array", Span::new(1, 0 + 1, 5 - 0)));
        ty.template_args.push(TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 6 + 1, 9 - 6)));
        assert_eq!(format!("{}", ty), "array<int>");
    }

    #[test]
    fn complex_type_display() {
        let mut ty = TypeExpr::named(Ident::new("array", Span::new(1, 6 + 1, 11 - 6)));
        ty.is_const = true;
        ty.template_args.push(TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 12 + 1, 15 - 12)));
        ty.suffixes.push(TypeSuffix::Array);
        ty.suffixes.push(TypeSuffix::Handle { is_const: true });
        assert_eq!(format!("{}", ty), "const array<int>[]@ const");
    }

    #[test]
    fn type_checks() {
        let mut ty = TypeExpr::named(Ident::new("MyClass", Span::new(1, 0 + 1, 7 - 0)));
        ty.suffixes.push(TypeSuffix::Handle { is_const: false });
        
        assert!(ty.has_handle());
        assert!(ty.is_reference_type());
        assert!(!ty.has_array());
        
        ty.suffixes.push(TypeSuffix::Array);
        assert!(ty.has_array());
    }

    #[test]
    fn void_type_check() {
        let ty = TypeExpr::primitive(PrimitiveType::Void, Span::new(1, 0 + 1, 4 - 0));
        assert!(ty.is_void());
    }

    #[test]
    fn all_primitive_type_sizes() {
        assert_eq!(PrimitiveType::Void.size_bytes(), 0);
        assert_eq!(PrimitiveType::Bool.size_bytes(), 1);
        assert_eq!(PrimitiveType::Int8.size_bytes(), 1);
        assert_eq!(PrimitiveType::UInt8.size_bytes(), 1);
        assert_eq!(PrimitiveType::Int16.size_bytes(), 2);
        assert_eq!(PrimitiveType::UInt16.size_bytes(), 2);
        assert_eq!(PrimitiveType::Int.size_bytes(), 4);
        assert_eq!(PrimitiveType::UInt.size_bytes(), 4);
        assert_eq!(PrimitiveType::Float.size_bytes(), 4);
        assert_eq!(PrimitiveType::Int64.size_bytes(), 8);
        assert_eq!(PrimitiveType::UInt64.size_bytes(), 8);
        assert_eq!(PrimitiveType::Double.size_bytes(), 8);
    }

    #[test]
    fn all_primitive_type_checks() {
        // Integer checks
        assert!(PrimitiveType::Int.is_integer());
        assert!(PrimitiveType::Int8.is_integer());
        assert!(PrimitiveType::Int16.is_integer());
        assert!(PrimitiveType::Int64.is_integer());
        assert!(PrimitiveType::UInt.is_integer());
        assert!(PrimitiveType::UInt8.is_integer());
        assert!(PrimitiveType::UInt16.is_integer());
        assert!(PrimitiveType::UInt64.is_integer());
        assert!(!PrimitiveType::Float.is_integer());
        assert!(!PrimitiveType::Double.is_integer());
        assert!(!PrimitiveType::Bool.is_integer());
        assert!(!PrimitiveType::Void.is_integer());

        // Float checks
        assert!(PrimitiveType::Float.is_float());
        assert!(PrimitiveType::Double.is_float());
        assert!(!PrimitiveType::Int.is_float());
        assert!(!PrimitiveType::Bool.is_float());

        // Signed checks
        assert!(PrimitiveType::Int.is_signed());
        assert!(PrimitiveType::Int8.is_signed());
        assert!(PrimitiveType::Int16.is_signed());
        assert!(PrimitiveType::Int64.is_signed());
        assert!(!PrimitiveType::UInt.is_signed());
        assert!(!PrimitiveType::Float.is_signed());

        // Unsigned checks
        assert!(PrimitiveType::UInt.is_unsigned());
        assert!(PrimitiveType::UInt8.is_unsigned());
        assert!(PrimitiveType::UInt16.is_unsigned());
        assert!(PrimitiveType::UInt64.is_unsigned());
        assert!(!PrimitiveType::Int.is_unsigned());
        assert!(!PrimitiveType::Float.is_unsigned());
    }

    #[test]
    fn all_primitive_type_display() {
        assert_eq!(format!("{}", PrimitiveType::Void), "void");
        assert_eq!(format!("{}", PrimitiveType::Bool), "bool");
        assert_eq!(format!("{}", PrimitiveType::Int), "int");
        assert_eq!(format!("{}", PrimitiveType::Int8), "int8");
        assert_eq!(format!("{}", PrimitiveType::Int16), "int16");
        assert_eq!(format!("{}", PrimitiveType::Int64), "int64");
        assert_eq!(format!("{}", PrimitiveType::UInt), "uint");
        assert_eq!(format!("{}", PrimitiveType::UInt8), "uint8");
        assert_eq!(format!("{}", PrimitiveType::UInt16), "uint16");
        assert_eq!(format!("{}", PrimitiveType::UInt64), "uint64");
        assert_eq!(format!("{}", PrimitiveType::Float), "float");
        assert_eq!(format!("{}", PrimitiveType::Double), "double");
    }

    #[test]
    fn type_base_display() {
        assert_eq!(format!("{}", TypeBase::Primitive(PrimitiveType::Int)), "int");
        assert_eq!(format!("{}", TypeBase::Named(Ident::new("Foo", Span::new(1, 1, 3)))), "Foo");
        assert_eq!(format!("{}", TypeBase::Auto), "auto");
        assert_eq!(format!("{}", TypeBase::Unknown), "?");
    }

    #[test]
    fn type_suffix_display() {
        assert_eq!(format!("{}", TypeSuffix::Array), "[]");
        assert_eq!(format!("{}", TypeSuffix::Handle { is_const: false }), "@");
        assert_eq!(format!("{}", TypeSuffix::Handle { is_const: true }), "@ const");
    }

    #[test]
    fn return_type_display() {
        let rt = ReturnType::new(
            TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            false,
            Span::new(1, 1, 3),
        );
        assert_eq!(format!("{}", rt), "int");

        let rt_ref = ReturnType::new(
            TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            true,
            Span::new(1, 1, 4),
        );
        assert_eq!(format!("{}", rt_ref), "int&");
    }

    #[test]
    fn param_type_display() {
        let pt = ParamType::new(
            TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            crate::ast::RefKind::None,
            Span::new(1, 1, 3),
        );
        assert_eq!(format!("{}", pt), "int");

        let pt_ref = ParamType::new(
            TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            crate::ast::RefKind::RefIn,
            Span::new(1, 1, 7),
        );
        assert!(format!("{}", pt_ref).contains("int"));
        assert!(format!("{}", pt_ref).contains("&"));
    }

    #[test]
    fn param_type_is_ref() {
        let pt = ParamType::new(
            TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            crate::ast::RefKind::None,
            Span::new(1, 1, 3),
        );
        assert!(!pt.is_ref());

        let pt_ref = ParamType::new(
            TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            crate::ast::RefKind::Ref,
            Span::new(1, 1, 4),
        );
        assert!(pt_ref.is_ref());
    }

    #[test]
    fn type_expr_with_scope() {
        let scope = Scope::new(
            false,
            vec![Ident::new("Namespace", Span::new(1, 1, 9))],
            Span::new(1, 1, 9),
        );
        let ty = TypeExpr::new(
            false,
            Some(scope.clone()),
            TypeBase::Named(Ident::new("Type", Span::new(1, 12, 4))),
            Vec::new(),
            Vec::new(),
            Span::new(1, 1, 16),
        );

        // Verify structure
        assert!(ty.scope.is_some());
        assert_eq!(ty.scope.as_ref().unwrap().segments.len(), 1);
        assert_eq!(ty.scope.as_ref().unwrap().segments[0].name, "Namespace");
        assert!(matches!(ty.base, TypeBase::Named(_)));

        // Also verify Display formatting
        assert_eq!(format!("{}", ty), "Namespace::Type");
    }

    #[test]
    fn type_expr_is_reference_type() {
        let mut ty = TypeExpr::named(Ident::new("MyClass", Span::new(1, 1, 7)));
        assert!(!ty.is_reference_type());

        ty.suffixes.push(TypeSuffix::Handle { is_const: false });
        assert!(ty.is_reference_type());
    }

    #[test]
    fn type_expr_non_void() {
        let ty = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3));
        assert!(!ty.is_void());
    }
}
