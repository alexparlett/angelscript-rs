//! Expression AST nodes for AngelScript.
//!
//! Provides nodes for all expression types including:
//! - Literals (numbers, strings, booleans)
//! - Binary operations (arithmetic, logical, comparison)
//! - Unary operations (negation, increment, etc.)
//! - Postfix operations (call, index, member access)
//! - Special expressions (cast, lambda)
//!
//! # Expression Precedence
//!
//! The parser uses Pratt parsing with the following precedence levels:
//! 1. Assignment (=, +=, etc.) - right associative
//! 2. Ternary (?:) - right associative
//! 3. Logical OR (||, ^^)
//! 4. Logical AND (&&)
//! 5. Bitwise OR (|)
//! 6. Bitwise XOR (^)
//! 7. Bitwise AND (&)
//! 8. Equality (==, !=, is, !is)
//! 9. Relational (<, <=, >, >=)
//! 10. Bitwise shift (<<, >>, >>>)
//! 11. Additive (+, -)
//! 12. Multiplicative (*, /, %)
//! 13. Power (**)
//! 15. Prefix unary (-, !, ~, ++, --, @)
//! 16. Postfix (call, index, member, ++, --)

use crate::ast::types::{ParamType, ReturnType, TypeExpr};
use crate::ast::{AssignOp, BinaryOp, Ident, PostfixOp, Scope, UnaryOp};
use angelscript_core::Span;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'ast> {
    /// Literal value
    Literal(LiteralExpr),
    /// Identifier reference
    Ident(IdentExpr<'ast>),
    /// Binary operation
    Binary(&'ast BinaryExpr<'ast>),
    /// Unary prefix operation
    Unary(&'ast UnaryExpr<'ast>),
    /// Assignment
    Assign(&'ast AssignExpr<'ast>),
    /// Ternary conditional (? :)
    Ternary(&'ast TernaryExpr<'ast>),
    /// Function call
    Call(&'ast CallExpr<'ast>),
    /// Array/object indexing
    Index(&'ast IndexExpr<'ast>),
    /// Member access (.)
    Member(&'ast MemberExpr<'ast>),
    /// Postfix operation (++ or --)
    Postfix(&'ast PostfixExpr<'ast>),
    /// Cast expression
    Cast(&'ast CastExpr<'ast>),
    /// Lambda (anonymous function)
    Lambda(&'ast LambdaExpr<'ast>),
    /// Initializer list
    InitList(InitListExpr<'ast>),
    /// Parenthesized expression
    Paren(&'ast ParenExpr<'ast>),
}

impl<'ast> Expr<'ast> {
    /// Get the span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Self::Literal(e) => e.span,
            Self::Ident(e) => e.span,
            Self::Binary(e) => e.span,
            Self::Unary(e) => e.span,
            Self::Assign(e) => e.span,
            Self::Ternary(e) => e.span,
            Self::Call(e) => e.span,
            Self::Index(e) => e.span,
            Self::Member(e) => e.span,
            Self::Postfix(e) => e.span,
            Self::Cast(e) => e.span,
            Self::Lambda(e) => e.span,
            Self::InitList(e) => e.span,
            Self::Paren(e) => e.span,
        }
    }
}

/// A literal value.
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralExpr {
    /// The literal kind
    pub kind: LiteralKind,
    /// Source location
    pub span: Span,
}

/// The kind of literal.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralKind {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f32),
    /// Double literal
    Double(f64),
    /// Boolean literal
    Bool(bool),
    /// String literal (raw bytes - factory interprets encoding)
    String(Vec<u8>),
    /// Null literal
    Null,
}

/// An identifier expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdentExpr<'ast> {
    /// Optional scope
    pub scope: Option<Scope<'ast>>,
    /// The identifier
    pub ident: Ident<'ast>,
    /// Type arguments for generic types/functions (e.g., `array<int>`, `Map<K,V>`)
    pub type_args: &'ast [crate::ast::types::TypeExpr<'ast>],
    /// Source location
    pub span: Span,
}

/// A binary operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinaryExpr<'ast> {
    /// Left operand
    pub left: &'ast Expr<'ast>,
    /// Operator
    pub op: BinaryOp,
    /// Right operand
    pub right: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// A unary prefix operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnaryExpr<'ast> {
    /// Operator
    pub op: UnaryOp,
    /// Operand
    pub operand: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// An assignment expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignExpr<'ast> {
    /// Left-hand side (target)
    pub target: &'ast Expr<'ast>,
    /// Assignment operator
    pub op: AssignOp,
    /// Right-hand side (value)
    pub value: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// A ternary conditional expression (condition ? then : else).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TernaryExpr<'ast> {
    /// Condition
    pub condition: &'ast Expr<'ast>,
    /// Then branch (if condition is true)
    pub then_expr: &'ast Expr<'ast>,
    /// Else branch (if condition is false)
    pub else_expr: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// A function call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CallExpr<'ast> {
    /// The function being called (can be any expression)
    pub callee: &'ast Expr<'ast>,
    /// Arguments
    pub args: &'ast [Argument<'ast>],
    /// Source location
    pub span: Span,
}

/// A function call argument.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Argument<'ast> {
    /// Optional named argument
    pub name: Option<Ident<'ast>>,
    /// Argument value
    pub value: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// Array or object indexing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndexExpr<'ast> {
    /// The object being indexed
    pub object: &'ast Expr<'ast>,
    /// Indices (can be multiple for multi-dimensional access)
    pub indices: &'ast [IndexItem<'ast>],
    /// Source location
    pub span: Span,
}

/// A single index item (can be named).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndexItem<'ast> {
    /// Optional name for associative arrays
    pub name: Option<Ident<'ast>>,
    /// Index expression
    pub index: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// Member access (dot operator).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemberExpr<'ast> {
    /// The object
    pub object: &'ast Expr<'ast>,
    /// The member being accessed
    pub member: MemberAccess<'ast>,
    /// Source location
    pub span: Span,
}

/// What is being accessed via the dot operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemberAccess<'ast> {
    /// Field access: obj.field
    Field(Ident<'ast>),
    /// Method call: obj.method(args)
    Method {
        name: Ident<'ast>,
        args: &'ast [Argument<'ast>],
    },
}

/// A postfix operation (++ or --).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostfixExpr<'ast> {
    /// The operand
    pub operand: &'ast Expr<'ast>,
    /// The operator
    pub op: PostfixOp,
    /// Source location
    pub span: Span,
}

/// A cast expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastExpr<'ast> {
    /// The target type
    pub target_type: TypeExpr<'ast>,
    /// The expression being cast
    pub expr: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// A lambda (anonymous function).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LambdaExpr<'ast> {
    /// Parameters
    pub params: &'ast [LambdaParam<'ast>],
    /// Return type (if specified)
    pub return_type: Option<ReturnType<'ast>>,
    /// Body (statement block)
    pub body: &'ast super::stmt::Block<'ast>,
    /// Source location
    pub span: Span,
}

/// A lambda parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LambdaParam<'ast> {
    /// Parameter type (optional)
    pub ty: Option<ParamType<'ast>>,
    /// Parameter name (optional for unused params)
    pub name: Option<Ident<'ast>>,
    /// Source location
    pub span: Span,
}

/// An initializer list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitListExpr<'ast> {
    /// Optional type annotation
    pub ty: Option<TypeExpr<'ast>>,
    /// Elements
    pub elements: &'ast [InitElement<'ast>],
    /// Source location
    pub span: Span,
}

/// An element in an initializer list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitElement<'ast> {
    /// Expression element
    Expr(&'ast Expr<'ast>),
    /// Nested initializer list
    InitList(InitListExpr<'ast>),
}

/// A parenthesized expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParenExpr<'ast> {
    /// The inner expression
    pub expr: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expr_span() {
        let lit = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::new(1, 0 + 1, 2 - 0),
        });
        assert_eq!(lit.span(), Span::new(1, 0 + 1, 2 - 0));
    }

    #[test]
    fn literal_kinds() {
        let int_lit = LiteralKind::Int(42);
        assert!(matches!(int_lit, LiteralKind::Int(42)));

        let bool_lit = LiteralKind::Bool(true);
        assert!(matches!(bool_lit, LiteralKind::Bool(true)));

        let null_lit = LiteralKind::Null;
        assert!(matches!(null_lit, LiteralKind::Null));
    }

    #[test]
    fn member_access_variants() {
        let field = MemberAccess::Field(Ident::new("x", Span::new(1, 0 + 1, 1 - 0)));
        assert!(matches!(field, MemberAccess::Field(_)));

        let method = MemberAccess::Method {
            name: Ident::new("foo", Span::new(1, 0 + 1, 3 - 0)),
            args: &[],
        };
        assert!(matches!(method, MemberAccess::Method { .. }));
    }

    #[test]
    fn all_expr_span_variants() {
        use crate::ast::types::TypeExpr;
        use bumpalo::Bump;

        let arena = Bump::new();

        // Literal
        let lit = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::new(1, 1, 2),
        });
        assert_eq!(lit.span(), Span::new(1, 1, 2));

        // Ident
        let ident = Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("x", Span::new(1, 1, 1)),
            type_args: &[],
            span: Span::new(1, 1, 1),
        });
        assert_eq!(ident.span(), Span::new(1, 1, 1));

        // Binary
        let left = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 1, 1),
        }));
        let right = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::new(1, 5, 1),
        }));
        let binary = Expr::Binary(arena.alloc(BinaryExpr {
            left,
            op: crate::ast::BinaryOp::Add,
            right,
            span: Span::new(1, 1, 5),
        }));
        assert_eq!(binary.span(), Span::new(1, 1, 5));

        // Unary
        let operand = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(5),
            span: Span::new(1, 2, 1),
        }));
        let unary = Expr::Unary(arena.alloc(UnaryExpr {
            op: crate::ast::UnaryOp::Neg,
            operand,
            span: Span::new(1, 1, 2),
        }));
        assert_eq!(unary.span(), Span::new(1, 1, 2));

        // Assign
        let target = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("x", Span::new(1, 1, 1)),
            type_args: &[],
            span: Span::new(1, 1, 1),
        }));
        let value = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(10),
            span: Span::new(1, 5, 2),
        }));
        let assign = Expr::Assign(arena.alloc(AssignExpr {
            target,
            op: crate::ast::AssignOp::Assign,
            value,
            span: Span::new(1, 1, 6),
        }));
        assert_eq!(assign.span(), Span::new(1, 1, 6));

        // Ternary
        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::new(1, 1, 4),
        }));
        let then_expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 8, 1),
        }));
        let else_expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::new(1, 12, 1),
        }));
        let ternary = Expr::Ternary(arena.alloc(TernaryExpr {
            condition,
            then_expr,
            else_expr,
            span: Span::new(1, 1, 12),
        }));
        assert_eq!(ternary.span(), Span::new(1, 1, 12));

        // Call
        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("foo", Span::new(1, 1, 3)),
            type_args: &[],
            span: Span::new(1, 1, 3),
        }));
        let call = Expr::Call(arena.alloc(CallExpr {
            callee,
            args: &[],
            span: Span::new(1, 1, 6),
        }));
        assert_eq!(call.span(), Span::new(1, 1, 6));

        // Index
        let object = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("arr", Span::new(1, 1, 3)),
            type_args: &[],
            span: Span::new(1, 1, 3),
        }));
        let index = Expr::Index(arena.alloc(IndexExpr {
            object,
            indices: &[],
            span: Span::new(1, 1, 6),
        }));
        assert_eq!(index.span(), Span::new(1, 1, 6));

        // Member
        let object = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("obj", Span::new(1, 1, 3)),
            type_args: &[],
            span: Span::new(1, 1, 3),
        }));
        let member = Expr::Member(arena.alloc(MemberExpr {
            object,
            member: MemberAccess::Field(Ident::new("x", Span::new(1, 5, 1))),
            span: Span::new(1, 1, 5),
        }));
        assert_eq!(member.span(), Span::new(1, 1, 5));

        // Postfix
        let operand = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("i", Span::new(1, 1, 1)),
            type_args: &[],
            span: Span::new(1, 1, 1),
        }));
        let postfix = Expr::Postfix(arena.alloc(PostfixExpr {
            operand,
            op: crate::ast::PostfixOp::PostInc,
            span: Span::new(1, 1, 3),
        }));
        assert_eq!(postfix.span(), Span::new(1, 1, 3));

        // Cast
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(10),
            span: Span::new(1, 12, 2),
        }));
        let cast = Expr::Cast(arena.alloc(CastExpr {
            target_type: TypeExpr::primitive(
                crate::ast::types::PrimitiveType::Float,
                Span::new(1, 6, 5),
            ),
            expr,
            span: Span::new(1, 1, 13),
        }));
        assert_eq!(cast.span(), Span::new(1, 1, 13));

        // Lambda
        let body = arena.alloc(crate::ast::stmt::Block {
            stmts: &[],
            span: Span::new(1, 10, 2),
        });
        let lambda = Expr::Lambda(arena.alloc(LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::new(1, 1, 12),
        }));
        assert_eq!(lambda.span(), Span::new(1, 1, 12));

        // InitList
        let init_list = Expr::InitList(InitListExpr {
            ty: None,
            elements: &[],
            span: Span::new(1, 1, 2),
        });
        assert_eq!(init_list.span(), Span::new(1, 1, 2));

        // Paren
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(5),
            span: Span::new(1, 2, 1),
        }));
        let paren = Expr::Paren(arena.alloc(ParenExpr {
            expr,
            span: Span::new(1, 1, 3),
        }));
        assert_eq!(paren.span(), Span::new(1, 1, 3));
    }

    #[test]
    fn all_literal_kind_variants() {
        let int_lit = LiteralKind::Int(42);
        assert!(matches!(int_lit, LiteralKind::Int(42)));

        let float_lit = LiteralKind::Float(3.14);
        assert!(matches!(float_lit, LiteralKind::Float(_)));

        let double_lit = LiteralKind::Double(2.718);
        assert!(matches!(double_lit, LiteralKind::Double(_)));

        let bool_lit = LiteralKind::Bool(false);
        assert!(matches!(bool_lit, LiteralKind::Bool(false)));

        let str_lit = LiteralKind::String(b"hello".to_vec());
        assert!(matches!(str_lit, LiteralKind::String(_)));

        let null_lit = LiteralKind::Null;
        assert!(matches!(null_lit, LiteralKind::Null));
    }

    #[test]
    fn argument_with_name() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let value = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(10),
            span: Span::new(1, 8, 2),
        }));
        let arg = Argument {
            name: Some(Ident::new("value", Span::new(1, 1, 5))),
            value,
            span: Span::new(1, 1, 9),
        };
        assert!(arg.name.is_some());
    }

    #[test]
    fn index_item_with_name() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let index = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::String(b"value".to_vec()),
            span: Span::new(1, 10, 7),
        }));
        let item = IndexItem {
            name: Some(Ident::new("key", Span::new(1, 5, 3))),
            index,
            span: Span::new(1, 5, 12),
        };
        assert!(item.name.is_some());
    }

    #[test]
    fn init_element_variants() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 1, 1),
        }));
        let expr_elem = InitElement::Expr(expr);
        assert!(matches!(expr_elem, InitElement::Expr(_)));

        let list_elem = InitElement::InitList(InitListExpr {
            ty: None,
            elements: &[],
            span: Span::new(1, 1, 2),
        });
        assert!(matches!(list_elem, InitElement::InitList(_)));
    }

    #[test]
    fn lambda_param_with_type() {
        use crate::ast::types::{ParamType, PrimitiveType, TypeExpr};

        let param = LambdaParam {
            ty: Some(ParamType::new(
                TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
                crate::ast::RefKind::None,
                Span::new(1, 1, 3),
            )),
            name: Some(Ident::new("x", Span::new(1, 5, 1))),
            span: Span::new(1, 1, 5),
        };
        assert!(param.ty.is_some());
        assert!(param.name.is_some());
    }

    #[test]
    fn lambda_param_no_type() {
        let param = LambdaParam {
            ty: None,
            name: Some(Ident::new("x", Span::new(1, 1, 1))),
            span: Span::new(1, 1, 1),
        };
        assert!(param.ty.is_none());
    }
}
