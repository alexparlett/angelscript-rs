//! Declaration AST nodes for AngelScript.
//!
//! Provides nodes for all top-level declarations including:
//! - Functions
//! - Classes and interfaces
//! - Enums
//! - Global variables
//! - Namespaces
//! - Typedefs and funcdefs
//! - Imports and mixins

use crate::ast::{DeclModifiers, FuncAttr, Ident, Visibility};
use crate::ast::expr::{Expr, IdentExpr};
use crate::ast::stmt::Block;
use crate::ast::types::{ParamType, ReturnType, TypeExpr};
use crate::lexer::Span;

/// A top-level item in a script.
#[derive(Debug, Clone, PartialEq)]
pub enum Item<'ast> {
    /// Function declaration
    Function(FunctionDecl<'ast>),
    /// Class declaration
    Class(ClassDecl<'ast>),
    /// Interface declaration
    Interface(InterfaceDecl<'ast>),
    /// Enum declaration
    Enum(EnumDecl<'ast>),
    /// Global variable declaration
    GlobalVar(GlobalVarDecl<'ast>),
    /// Namespace declaration
    Namespace(NamespaceDecl<'ast>),
    /// Typedef declaration
    Typedef(TypedefDecl<'ast>),
    /// Funcdef declaration
    Funcdef(FuncdefDecl<'ast>),
    /// Mixin declaration
    Mixin(MixinDecl<'ast>),
    /// Import statement
    Import(ImportDecl<'ast>),
    /// Using namespace directive
    UsingNamespace(UsingNamespaceDecl<'ast>),
}

impl<'ast> Item<'ast> {
    /// Get the span of this item.
    pub fn span(&self) -> Span {
        match self {
            Self::Function(d) => d.span,
            Self::Class(d) => d.span,
            Self::Interface(d) => d.span,
            Self::Enum(d) => d.span,
            Self::GlobalVar(d) => d.span,
            Self::Namespace(d) => d.span,
            Self::Typedef(d) => d.span,
            Self::Funcdef(d) => d.span,
            Self::Mixin(d) => d.span,
            Self::Import(d) => d.span,
            Self::UsingNamespace(d) => d.span,
        }
    }
}

/// A function declaration.
///
/// Examples:
/// - `void foo() { }`
/// - `int add(int a, int b) { return a + b; }`
/// - `void method() const { }`
/// - `~MyClass() { }` (destructor)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionDecl<'ast> {
    /// Declaration modifiers (shared, external)
    pub modifiers: DeclModifiers,
    /// Visibility (for class members)
    pub visibility: Visibility,
    /// Return type (None for constructors/destructors)
    pub return_type: Option<ReturnType<'ast>>,
    /// Function name
    pub name: Ident<'ast>,
    /// Template parameters (for application-registered template functions)
    /// Example: swap<T> has template_params = ["T"]
    pub template_params: &'ast [Ident<'ast>],
    /// Parameters
    pub params: &'ast [FunctionParam<'ast>],
    /// Whether this is a const method
    pub is_const: bool,
    /// Function attributes (override, final, etc.)
    pub attrs: FuncAttr,
    /// Body (None for declarations without implementation)
    pub body: Option<Block<'ast>>,
    /// Whether this is a destructor
    pub is_destructor: bool,
    /// Source location
    pub span: Span,
}

impl<'ast> FunctionDecl<'ast> {
    /// Check if this is a constructor.
    pub fn is_constructor(&self) -> bool {
        self.return_type.is_none() && !self.is_destructor
    }
}

/// A function parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionParam<'ast> {
    /// Parameter type
    pub ty: ParamType<'ast>,
    /// Parameter name (optional for interface methods)
    pub name: Option<Ident<'ast>>,
    /// Default value
    pub default: Option<&'ast Expr<'ast>>,
    /// Whether this is a variadic parameter (...)
    pub is_variadic: bool,
    /// Source location
    pub span: Span,
}

/// A class declaration.
///
/// Example:
/// ```as
/// class Player : Enemy, IDrawable {
///     private int health = 100;
///
///     void takeDamage(int amount) {
///         health -= amount;
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassDecl<'ast> {
    /// Declaration modifiers (shared, abstract, final, external)
    pub modifiers: DeclModifiers,
    /// Class name
    pub name: Ident<'ast>,
    /// Template parameters (for application-registered template classes)
    /// Example: Container<T> has template_params = ["T"]
    pub template_params: &'ast [Ident<'ast>],
    /// Base class and interfaces (supports scoped names like Namespace::Interface)
    pub inheritance: &'ast [IdentExpr<'ast>],
    /// Class members
    pub members: &'ast [ClassMember<'ast>],
    /// Source location
    pub span: Span,
}

/// A class member.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClassMember<'ast> {
    /// Method
    Method(FunctionDecl<'ast>),
    /// Field (variable)
    Field(FieldDecl<'ast>),
    /// Virtual property
    VirtualProperty(VirtualPropertyDecl<'ast>),
    /// Nested funcdef
    Funcdef(FuncdefDecl<'ast>),
}

/// A field declaration in a class.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldDecl<'ast> {
    /// Visibility
    pub visibility: Visibility,
    /// Field type
    pub ty: TypeExpr<'ast>,
    /// Field name
    pub name: Ident<'ast>,
    /// Optional initializer
    pub init: Option<&'ast Expr<'ast>>,
    /// Source location
    pub span: Span,
}

/// A virtual property declaration.
///
/// Example:
/// ```as
/// private int health {
///     get const { return _health; }
///     set { _health = value; }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualPropertyDecl<'ast> {
    /// Visibility
    pub visibility: Visibility,
    /// Property type
    pub ty: ReturnType<'ast>,
    /// Property name
    pub name: Ident<'ast>,
    /// Accessors (get/set)
    pub accessors: &'ast [PropertyAccessor<'ast>],
    /// Source location
    pub span: Span,
}

/// A property accessor (get or set).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyAccessor<'ast> {
    /// Accessor kind (get or set)
    pub kind: crate::ast::PropertyAccessorKind,
    /// Whether this accessor is const
    pub is_const: bool,
    /// Function attributes
    pub attrs: FuncAttr,
    /// Body (None for interface)
    pub body: Option<Block<'ast>>,
    /// Source location
    pub span: Span,
}

/// An interface declaration.
///
/// Example:
/// ```as
/// interface IDrawable : IRenderable {
///     void draw();
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterfaceDecl<'ast> {
    /// Declaration modifiers (external, shared)
    pub modifiers: DeclModifiers,
    /// Interface name
    pub name: Ident<'ast>,
    /// Base interfaces
    pub bases: &'ast [Ident<'ast>],
    /// Interface members
    pub members: &'ast [InterfaceMember<'ast>],
    /// Source location
    pub span: Span,
}

/// An interface member.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterfaceMember<'ast> {
    /// Method signature
    Method(InterfaceMethod<'ast>),
    /// Virtual property
    VirtualProperty(VirtualPropertyDecl<'ast>),
}

/// An interface method signature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterfaceMethod<'ast> {
    /// Return type
    pub return_type: ReturnType<'ast>,
    /// Method name
    pub name: Ident<'ast>,
    /// Parameters
    pub params: &'ast [FunctionParam<'ast>],
    /// Whether this is a const method
    pub is_const: bool,
    /// Source location
    pub span: Span,
}

/// An enum declaration.
///
/// Example:
/// ```as
/// enum Color {
///     Red,
///     Green = 1,
///     Blue
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnumDecl<'ast> {
    /// Declaration modifiers (shared, external)
    pub modifiers: DeclModifiers,
    /// Enum name
    pub name: Ident<'ast>,
    /// Enumerators
    pub enumerators: &'ast [Enumerator<'ast>],
    /// Source location
    pub span: Span,
}

/// An enumerator (enum value).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Enumerator<'ast> {
    /// Enumerator name
    pub name: Ident<'ast>,
    /// Optional value
    pub value: Option<&'ast Expr<'ast>>,
    /// Source location
    pub span: Span,
}

/// A global variable declaration.
///
/// Example: `int globalCounter = 0;`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalVarDecl<'ast> {
    /// Visibility
    pub visibility: Visibility,
    /// Variable type
    pub ty: TypeExpr<'ast>,
    /// Variable name
    pub name: Ident<'ast>,
    /// Optional initializer
    pub init: Option<&'ast Expr<'ast>>,
    /// Source location
    pub span: Span,
}

/// A namespace declaration.
///
/// Example:
/// ```as
/// namespace Game {
///     class Entity { }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NamespaceDecl<'ast> {
    /// Namespace path (can be nested: A::B::C)
    pub path: &'ast [Ident<'ast>],
    /// Namespace contents
    pub items: &'ast [Item<'ast>],
    /// Source location
    pub span: Span,
}

/// A using namespace directive.
///
/// Example:
/// ```as
/// using namespace Game::Utils;
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsingNamespaceDecl<'ast> {
    /// Namespace path to import (e.g., ["Game", "Utils"])
    pub path: &'ast [Ident<'ast>],
    /// Source location
    pub span: Span,
}

/// A typedef declaration.
///
/// Example: `typedef int EntityId;`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypedefDecl<'ast> {
    /// Base type (must be primitive)
    pub base_type: TypeExpr<'ast>,
    /// New type name
    pub name: Ident<'ast>,
    /// Source location
    pub span: Span,
}

/// A funcdef declaration (function signature type).
///
/// Example: `funcdef void Callback(int);`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuncdefDecl<'ast> {
    /// Declaration modifiers (external, shared)
    pub modifiers: DeclModifiers,
    /// Return type
    pub return_type: ReturnType<'ast>,
    /// Funcdef name
    pub name: Ident<'ast>,
    /// Template parameters (for application-registered template funcdefs)
    /// Example: Callback<T> has template_params = ["T"]
    pub template_params: &'ast [Ident<'ast>],
    /// Parameters
    pub params: &'ast [FunctionParam<'ast>],
    /// Source location
    pub span: Span,
}

/// A mixin declaration.
///
/// Example: `mixin class MyMixin { }`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixinDecl<'ast> {
    /// The class being declared as a mixin
    pub class: ClassDecl<'ast>,
    /// Source location
    pub span: Span,
}

/// An import declaration.
///
/// Example: `import void func(int) from "module";`
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl<'ast> {
    /// Return type
    pub return_type: ReturnType<'ast>,
    /// Function name
    pub name: Ident<'ast>,
    /// Parameters
    pub params: &'ast [FunctionParam<'ast>],
    /// Function attributes
    pub attrs: FuncAttr,
    /// Module to import from
    pub module: String,
    /// Source location
    pub span: Span,
}

/// A function signature declaration (for FFI registration).
///
/// This is used for parsing FFI declaration strings like "int add(int a, int b)".
/// Unlike `FunctionDecl`, this contains only the signature without body,
/// modifiers, or visibility.
///
/// # Example
///
/// ```ignore
/// use angelscript::parse_ffi_function_decl;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let sig = parse_ffi_function_decl("int add(int a, int b)", &arena)?;
/// assert_eq!(sig.name.name, "add");
/// assert_eq!(sig.params.len(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionSignatureDecl<'ast> {
    /// Return type
    pub return_type: ReturnType<'ast>,
    /// Function name
    pub name: Ident<'ast>,
    /// Parameters
    pub params: &'ast [FunctionParam<'ast>],
    /// Whether this is a const method
    pub is_const: bool,
    /// Function attributes (property, etc.)
    pub attrs: FuncAttr,
    /// Source location
    pub span: Span,
}

/// A property declaration (for FFI registration).
///
/// This is used for parsing FFI declaration strings like "int score" or "const string name".
/// Unlike `GlobalVarDecl`, this contains only the type and name without initializer.
///
/// # Example
///
/// ```ignore
/// use angelscript::parse_property_decl;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let prop = parse_property_decl("const int score", &arena)?;
/// assert_eq!(prop.name.name, "score");
/// assert!(prop.ty.is_const);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyDecl<'ast> {
    /// Property type (includes const modifier)
    pub ty: TypeExpr<'ast>,
    /// Property name
    pub name: Ident<'ast>,
    /// Source location
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_is_constructor() {
        let func = FunctionDecl {
            modifiers: DeclModifiers::new(),
            visibility: Visibility::Public,
            return_type: None,
            name: Ident::new("MyClass", Span::new(1, 0 + 1, 7 - 0)),
            template_params: &[],
            params: &[],
            is_const: false,
            attrs: FuncAttr::new(),
            body: None,
            is_destructor: false,
            span: Span::new(1, 0 + 1, 7 - 0),
        };
        assert!(func.is_constructor());
    }

    #[test]
    fn function_is_destructor() {
        let func = FunctionDecl {
            modifiers: DeclModifiers::new(),
            visibility: Visibility::Public,
            return_type: None,
            name: Ident::new("MyClass", Span::new(1, 0 + 1, 7 - 0)),
            template_params: &[],
            params: &[],
            is_const: false,
            attrs: FuncAttr::new(),
            body: None,
            is_destructor: true,
            span: Span::new(1, 0 + 1, 7 - 0),
        };
        assert!(func.is_destructor);
        assert!(!func.is_constructor());
    }

    #[test]
    fn item_span() {
        let func = FunctionDecl {
            modifiers: DeclModifiers::new(),
            visibility: Visibility::Public,
            return_type: None,
            name: Ident::new("foo", Span::new(1, 0 + 1, 3 - 0)),
            template_params: &[],
            params: &[],
            is_const: false,
            attrs: FuncAttr::new(),
            body: None,
            is_destructor: false,
            span: Span::new(1, 0 + 1, 10 - 0),
        };
        let item = Item::Function(func);
        assert_eq!(item.span(), Span::new(1, 0 + 1, 10 - 0));
    }

    #[test]
    fn all_item_span_variants() {
        use crate::ast::types::{TypeExpr, PrimitiveType, ReturnType};

        // Function
        let func_item = Item::Function(FunctionDecl {
            modifiers: DeclModifiers::new(),
            visibility: Visibility::Public,
            return_type: Some(ReturnType::new(
                TypeExpr::primitive(PrimitiveType::Void, Span::new(1, 1, 4)),
                false,
                Span::new(1, 1, 4),
            )),
            name: Ident::new("test", Span::new(1, 6, 4)),
            template_params: &[],
            params: &[],
            is_const: false,
            attrs: FuncAttr::new(),
            body: None,
            is_destructor: false,
            span: Span::new(1, 1, 15),
        });
        assert_eq!(func_item.span(), Span::new(1, 1, 15));

        // Class
        let class_item = Item::Class(ClassDecl {
            modifiers: DeclModifiers::new(),
            name: Ident::new("MyClass", Span::new(1, 7, 7)),
            template_params: &[],
            inheritance: &[],
            members: &[],
            span: Span::new(1, 1, 20),
        });
        assert_eq!(class_item.span(), Span::new(1, 1, 20));

        // Interface
        let interface_item = Item::Interface(InterfaceDecl {
            modifiers: DeclModifiers::new(),
            name: Ident::new("IFoo", Span::new(1, 11, 4)),
            bases: &[],
            members: &[],
            span: Span::new(1, 1, 20),
        });
        assert_eq!(interface_item.span(), Span::new(1, 1, 20));

        // Enum
        let enum_item = Item::Enum(EnumDecl {
            modifiers: DeclModifiers::new(),
            name: Ident::new("Color", Span::new(1, 6, 5)),
            enumerators: &[],
            span: Span::new(1, 1, 15),
        });
        assert_eq!(enum_item.span(), Span::new(1, 1, 15));

        // GlobalVar
        let var_item = Item::GlobalVar(GlobalVarDecl {
            visibility: Visibility::Public,
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            name: Ident::new("g_value", Span::new(1, 5, 7)),
            init: None,
            span: Span::new(1, 1, 13),
        });
        assert_eq!(var_item.span(), Span::new(1, 1, 13));

        // Namespace
        let arena = bumpalo::Bump::new();
        let ns_item = Item::Namespace(NamespaceDecl {
            path: bumpalo::vec![in &arena; Ident::new("Game", Span::new(1, 11, 4))].into_bump_slice(),
            items: &[],
            span: Span::new(1, 1, 20),
        });
        assert_eq!(ns_item.span(), Span::new(1, 1, 20));

        // Typedef
        let typedef_item = Item::Typedef(TypedefDecl {
            base_type: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 9, 3)),
            name: Ident::new("MyInt", Span::new(1, 13, 5)),
            span: Span::new(1, 1, 18),
        });
        assert_eq!(typedef_item.span(), Span::new(1, 1, 18));

        // Funcdef
        let funcdef_item = Item::Funcdef(FuncdefDecl {
            modifiers: DeclModifiers::new(),
            return_type: ReturnType::new(
                TypeExpr::primitive(PrimitiveType::Void, Span::new(1, 9, 4)),
                false,
                Span::new(1, 9, 4),
            ),
            name: Ident::new("Callback", Span::new(1, 14, 8)),
            template_params: &[],
            params: &[],
            span: Span::new(1, 1, 24),
        });
        assert_eq!(funcdef_item.span(), Span::new(1, 1, 24));

        // Mixin
        let mixin_item = Item::Mixin(MixinDecl {
            class: ClassDecl {
                modifiers: DeclModifiers::new(),
                name: Ident::new("MyMixin", Span::new(1, 12, 7)),
                template_params: &[],
                inheritance: &[],
                members: &[],
                span: Span::new(1, 6, 20),
            },
            span: Span::new(1, 1, 25),
        });
        assert_eq!(mixin_item.span(), Span::new(1, 1, 25));

        // Import
        let import_item = Item::Import(ImportDecl {
            return_type: ReturnType::new(
                TypeExpr::primitive(PrimitiveType::Void, Span::new(1, 8, 4)),
                false,
                Span::new(1, 8, 4),
            ),
            name: Ident::new("func", Span::new(1, 13, 4)),
            params: &[],
            attrs: FuncAttr::new(),
            module: "module".to_string(),
            span: Span::new(1, 1, 30),
        });
        assert_eq!(import_item.span(), Span::new(1, 1, 30));
    }

    #[test]
    fn function_with_body() {
        use crate::ast::stmt::Block;

        let func = FunctionDecl {
            modifiers: DeclModifiers::new(),
            visibility: Visibility::Public,
            return_type: None,
            name: Ident::new("MyClass", Span::new(1, 1, 7)),
            template_params: &[],
            params: &[],
            is_const: false,
            attrs: FuncAttr::new(),
            body: Some(Block {
                stmts: &[],
                span: Span::new(1, 10, 2),
            }),
            is_destructor: false,
            span: Span::new(1, 1, 12),
        };
        assert!(func.body.is_some());
        assert!(func.is_constructor());
    }

    #[test]
    fn function_param_with_default() {
        use crate::ast::types::{ParamType, TypeExpr, PrimitiveType};
        use crate::ast::expr::{Expr, LiteralExpr, LiteralKind};
        use bumpalo::Bump;
        let arena = Bump::new();

        let param = FunctionParam {
            ty: ParamType::new(
                TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
                crate::ast::RefKind::None,
                Span::new(1, 1, 3),
            ),
            name: Some(Ident::new("x", Span::new(1, 5, 1))),
            default: Some(arena.alloc(Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(10),
                span: Span::new(1, 9, 2),
            }))),
            is_variadic: false,
            span: Span::new(1, 1, 10),
        };
        assert!(param.default.is_some());
        assert!(!param.is_variadic);
    }

    #[test]
    fn function_param_variadic() {
        use crate::ast::types::{ParamType, TypeExpr, PrimitiveType};

        let param = FunctionParam {
            ty: ParamType::new(
                TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
                crate::ast::RefKind::None,
                Span::new(1, 1, 3),
            ),
            name: None,
            default: None,
            is_variadic: true,
            span: Span::new(1, 1, 6),
        };
        assert!(param.is_variadic);
        assert!(param.name.is_none());
    }

    #[test]
    fn class_member_variants() {
        use crate::ast::types::{TypeExpr, PrimitiveType};

        let method = ClassMember::Method(FunctionDecl {
            modifiers: DeclModifiers::new(),
            visibility: Visibility::Public,
            return_type: None,
            name: Ident::new("foo", Span::new(1, 1, 3)),
            template_params: &[],
            params: &[],
            is_const: false,
            attrs: FuncAttr::new(),
            body: None,
            is_destructor: false,
            span: Span::new(1, 1, 10),
        });
        assert!(matches!(method, ClassMember::Method(_)));

        let field = ClassMember::Field(FieldDecl {
            visibility: Visibility::Private,
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 9, 3)),
            name: Ident::new("value", Span::new(1, 13, 5)),
            init: None,
            span: Span::new(1, 1, 18),
        });
        assert!(matches!(field, ClassMember::Field(_)));
    }

    #[test]
    fn interface_member_variants() {
        use crate::ast::types::{ReturnType, TypeExpr, PrimitiveType};

        let method = InterfaceMember::Method(InterfaceMethod {
            return_type: ReturnType::new(
                TypeExpr::primitive(PrimitiveType::Void, Span::new(1, 1, 4)),
                false,
                Span::new(1, 1, 4),
            ),
            name: Ident::new("draw", Span::new(1, 6, 4)),
            params: &[],
            is_const: false,
            span: Span::new(1, 1, 12),
        });
        assert!(matches!(method, InterfaceMember::Method(_)));
    }

    #[test]
    fn enumerator_with_value() {
        use crate::ast::expr::{Expr, LiteralExpr, LiteralKind};
        use bumpalo::Bump;
        let arena = Bump::new();

        let enumerator = Enumerator {
            name: Ident::new("Red", Span::new(1, 1, 3)),
            value: Some(arena.alloc(Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(1),
                span: Span::new(1, 7, 1),
            }))),
            span: Span::new(1, 1, 7),
        };
        assert!(enumerator.value.is_some());
    }

    #[test]
    fn field_with_init() {
        use crate::ast::types::{TypeExpr, PrimitiveType};
        use crate::ast::expr::{Expr, LiteralExpr, LiteralKind};
        use bumpalo::Bump;
        let arena = Bump::new();

        let field = FieldDecl {
            visibility: Visibility::Private,
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 9, 3)),
            name: Ident::new("count", Span::new(1, 13, 5)),
            init: Some(arena.alloc(Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(0),
                span: Span::new(1, 21, 1),
            }))),
            span: Span::new(1, 1, 22),
        };
        assert!(field.init.is_some());
    }

    #[test]
    fn property_accessor_structure() {
        use crate::ast::stmt::Block;

        let accessor = PropertyAccessor {
            kind: crate::ast::PropertyAccessorKind::Get,
            is_const: true,
            attrs: FuncAttr::new(),
            body: Some(Block {
                stmts: &[],
                span: Span::new(1, 15, 2),
            }),
            span: Span::new(1, 5, 12),
        };
        assert!(accessor.is_const);
        assert!(accessor.body.is_some());
    }

    #[test]
    fn namespace_with_nested_path() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let ns = NamespaceDecl {
            path: bumpalo::vec![in &arena;
                Ident::new("Game", Span::new(1, 11, 4)),
                Ident::new("Utils", Span::new(1, 17, 5)),
            ].into_bump_slice(),
            items: &[],
            span: Span::new(1, 1, 30),
        };
        assert_eq!(ns.path.len(), 2);
    }
}
