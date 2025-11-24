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

use crate::ast::{AssignOp, BinaryOp, Ident, PostfixOp, Scope, UnaryOp};
use crate::ast::types::{ParamType, ReturnType, TypeExpr};
use crate::lexer::Span;

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value
    Literal(LiteralExpr),
    /// Identifier reference
    Ident(IdentExpr),
    /// Binary operation
    Binary(Box<BinaryExpr>),
    /// Unary prefix operation
    Unary(Box<UnaryExpr>),
    /// Assignment
    Assign(Box<AssignExpr>),
    /// Ternary conditional (? :)
    Ternary(Box<TernaryExpr>),
    /// Function call
    Call(Box<CallExpr>),
    /// Array/object indexing
    Index(Box<IndexExpr>),
    /// Member access (.)
    Member(Box<MemberExpr>),
    /// Postfix operation (++ or --)
    Postfix(Box<PostfixExpr>),
    /// Cast expression
    Cast(Box<CastExpr>),
    /// Lambda (anonymous function)
    Lambda(Box<LambdaExpr>),
    /// Initializer list
    InitList(InitListExpr),
    /// Parenthesized expression
    Paren(Box<ParenExpr>),
}

impl Expr {
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
    /// String literal
    String(String),
    /// Null literal
    Null,
}

/// An identifier expression.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentExpr {
    /// Optional scope
    pub scope: Option<Scope>,
    /// The identifier
    pub ident: Ident,
    /// Source location
    pub span: Span,
}

/// A binary operation.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr {
    /// Left operand
    pub left: Expr,
    /// Operator
    pub op: BinaryOp,
    /// Right operand
    pub right: Expr,
    /// Source location
    pub span: Span,
}

/// A unary prefix operation.
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    /// Operator
    pub op: UnaryOp,
    /// Operand
    pub operand: Expr,
    /// Source location
    pub span: Span,
}

/// An assignment expression.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignExpr {
    /// Left-hand side (target)
    pub target: Expr,
    /// Assignment operator
    pub op: AssignOp,
    /// Right-hand side (value)
    pub value: Expr,
    /// Source location
    pub span: Span,
}

/// A ternary conditional expression (condition ? then : else).
#[derive(Debug, Clone, PartialEq)]
pub struct TernaryExpr {
    /// Condition
    pub condition: Expr,
    /// Then branch (if condition is true)
    pub then_expr: Expr,
    /// Else branch (if condition is false)
    pub else_expr: Expr,
    /// Source location
    pub span: Span,
}

/// A function call.
#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    /// The function being called (can be any expression)
    pub callee: Expr,
    /// Arguments
    pub args: Vec<Argument>,
    /// Source location
    pub span: Span,
}

/// A function call argument.
#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    /// Optional named argument
    pub name: Option<Ident>,
    /// Argument value
    pub value: Expr,
    /// Source location
    pub span: Span,
}

/// Array or object indexing.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    /// The object being indexed
    pub object: Expr,
    /// Indices (can be multiple for multi-dimensional access)
    pub indices: Vec<IndexItem>,
    /// Source location
    pub span: Span,
}

/// A single index item (can be named).
#[derive(Debug, Clone, PartialEq)]
pub struct IndexItem {
    /// Optional name for associative arrays
    pub name: Option<Ident>,
    /// Index expression
    pub index: Expr,
    /// Source location
    pub span: Span,
}

/// Member access (dot operator).
#[derive(Debug, Clone, PartialEq)]
pub struct MemberExpr {
    /// The object
    pub object: Expr,
    /// The member being accessed
    pub member: MemberAccess,
    /// Source location
    pub span: Span,
}

/// What is being accessed via the dot operator.
#[derive(Debug, Clone, PartialEq)]
pub enum MemberAccess {
    /// Field access: obj.field
    Field(Ident),
    /// Method call: obj.method(args)
    Method { name: Ident, args: Vec<Argument> },
}

/// A postfix operation (++ or --).
#[derive(Debug, Clone, PartialEq)]
pub struct PostfixExpr {
    /// The operand
    pub operand: Expr,
    /// The operator
    pub op: PostfixOp,
    /// Source location
    pub span: Span,
}

/// A cast expression.
#[derive(Debug, Clone, PartialEq)]
pub struct CastExpr {
    /// The target type
    pub target_type: TypeExpr,
    /// The expression being cast
    pub expr: Expr,
    /// Source location
    pub span: Span,
}

/// A lambda (anonymous function).
#[derive(Debug, Clone, PartialEq)]
pub struct LambdaExpr {
    /// Parameters
    pub params: Vec<LambdaParam>,
    /// Return type (if specified)
    pub return_type: Option<ReturnType>,
    /// Body (statement block)
    pub body: Box<super::stmt::Block>,
    /// Source location
    pub span: Span,
}

/// A lambda parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    /// Parameter type (optional)
    pub ty: Option<ParamType>,
    /// Parameter name (optional for unused params)
    pub name: Option<Ident>,
    /// Source location
    pub span: Span,
}

/// An initializer list.
#[derive(Debug, Clone, PartialEq)]
pub struct InitListExpr {
    /// Optional type annotation
    pub ty: Option<TypeExpr>,
    /// Elements
    pub elements: Vec<InitElement>,
    /// Source location
    pub span: Span,
}

/// An element in an initializer list.
#[derive(Debug, Clone, PartialEq)]
pub enum InitElement {
    /// Expression element
    Expr(Expr),
    /// Nested initializer list
    InitList(InitListExpr),
}

/// A parenthesized expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ParenExpr {
    /// The inner expression
    pub expr: Expr,
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
            args: Vec::new(),
        };
        assert!(matches!(method, MemberAccess::Method { .. }));
    }

    #[test]
    fn all_expr_span_variants() {
        use crate::ast::types::TypeExpr;

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
            span: Span::new(1, 1, 1),
        });
        assert_eq!(ident.span(), Span::new(1, 1, 1));

        // Binary
        let binary = Expr::Binary(Box::new(BinaryExpr {
            left: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(1),
                span: Span::new(1, 1, 1),
            }),
            op: crate::ast::BinaryOp::Add,
            right: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(2),
                span: Span::new(1, 5, 1),
            }),
            span: Span::new(1, 1, 5),
        }));
        assert_eq!(binary.span(), Span::new(1, 1, 5));

        // Unary
        let unary = Expr::Unary(Box::new(UnaryExpr {
            op: crate::ast::UnaryOp::Neg,
            operand: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(5),
                span: Span::new(1, 2, 1),
            }),
            span: Span::new(1, 1, 2),
        }));
        assert_eq!(unary.span(), Span::new(1, 1, 2));

        // Assign
        let assign = Expr::Assign(Box::new(AssignExpr {
            target: Expr::Ident(IdentExpr {
                scope: None,
                ident: Ident::new("x", Span::new(1, 1, 1)),
                span: Span::new(1, 1, 1),
            }),
            op: crate::ast::AssignOp::Assign,
            value: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(10),
                span: Span::new(1, 5, 2),
            }),
            span: Span::new(1, 1, 6),
        }));
        assert_eq!(assign.span(), Span::new(1, 1, 6));

        // Ternary
        let ternary = Expr::Ternary(Box::new(TernaryExpr {
            condition: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Bool(true),
                span: Span::new(1, 1, 4),
            }),
            then_expr: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(1),
                span: Span::new(1, 8, 1),
            }),
            else_expr: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(2),
                span: Span::new(1, 12, 1),
            }),
            span: Span::new(1, 1, 12),
        }));
        assert_eq!(ternary.span(), Span::new(1, 1, 12));

        // Call
        let call = Expr::Call(Box::new(CallExpr {
            callee: Expr::Ident(IdentExpr {
                scope: None,
                ident: Ident::new("foo", Span::new(1, 1, 3)),
                span: Span::new(1, 1, 3),
            }),
            args: Vec::new(),
            span: Span::new(1, 1, 6),
        }));
        assert_eq!(call.span(), Span::new(1, 1, 6));

        // Index
        let index = Expr::Index(Box::new(IndexExpr {
            object: Expr::Ident(IdentExpr {
                scope: None,
                ident: Ident::new("arr", Span::new(1, 1, 3)),
                span: Span::new(1, 1, 3),
            }),
            indices: Vec::new(),
            span: Span::new(1, 1, 6),
        }));
        assert_eq!(index.span(), Span::new(1, 1, 6));

        // Member
        let member = Expr::Member(Box::new(MemberExpr {
            object: Expr::Ident(IdentExpr {
                scope: None,
                ident: Ident::new("obj", Span::new(1, 1, 3)),
                span: Span::new(1, 1, 3),
            }),
            member: MemberAccess::Field(Ident::new("x", Span::new(1, 5, 1))),
            span: Span::new(1, 1, 5),
        }));
        assert_eq!(member.span(), Span::new(1, 1, 5));

        // Postfix
        let postfix = Expr::Postfix(Box::new(PostfixExpr {
            operand: Expr::Ident(IdentExpr {
                scope: None,
                ident: Ident::new("i", Span::new(1, 1, 1)),
                span: Span::new(1, 1, 1),
            }),
            op: crate::ast::PostfixOp::PostInc,
            span: Span::new(1, 1, 3),
        }));
        assert_eq!(postfix.span(), Span::new(1, 1, 3));

        // Cast
        let cast = Expr::Cast(Box::new(CastExpr {
            target_type: TypeExpr::primitive(crate::ast::types::PrimitiveType::Float, Span::new(1, 6, 5)),
            expr: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(10),
                span: Span::new(1, 12, 2),
            }),
            span: Span::new(1, 1, 13),
        }));
        assert_eq!(cast.span(), Span::new(1, 1, 13));

        // Lambda
        let lambda = Expr::Lambda(Box::new(LambdaExpr {
            params: Vec::new(),
            return_type: None,
            body: Box::new(crate::ast::stmt::Block {
                stmts: Vec::new(),
                span: Span::new(1, 10, 2),
            }),
            span: Span::new(1, 1, 12),
        }));
        assert_eq!(lambda.span(), Span::new(1, 1, 12));

        // InitList
        let init_list = Expr::InitList(InitListExpr {
            ty: None,
            elements: Vec::new(),
            span: Span::new(1, 1, 2),
        });
        assert_eq!(init_list.span(), Span::new(1, 1, 2));

        // Paren
        let paren = Expr::Paren(Box::new(ParenExpr {
            expr: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(5),
                span: Span::new(1, 2, 1),
            }),
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

        let str_lit = LiteralKind::String("hello".to_string());
        assert!(matches!(str_lit, LiteralKind::String(_)));

        let null_lit = LiteralKind::Null;
        assert!(matches!(null_lit, LiteralKind::Null));
    }

    #[test]
    fn argument_with_name() {
        let arg = Argument {
            name: Some(Ident::new("value", Span::new(1, 1, 5))),
            value: Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(10),
                span: Span::new(1, 8, 2),
            }),
            span: Span::new(1, 1, 9),
        };
        assert!(arg.name.is_some());
    }

    #[test]
    fn index_item_with_name() {
        let item = IndexItem {
            name: Some(Ident::new("key", Span::new(1, 5, 3))),
            index: Expr::Literal(LiteralExpr {
                kind: LiteralKind::String("value".to_string()),
                span: Span::new(1, 10, 7),
            }),
            span: Span::new(1, 5, 12),
        };
        assert!(item.name.is_some());
    }

    #[test]
    fn init_element_variants() {
        let expr_elem = InitElement::Expr(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::new(1, 1, 1),
        }));
        assert!(matches!(expr_elem, InitElement::Expr(_)));

        let list_elem = InitElement::InitList(InitListExpr {
            ty: None,
            elements: Vec::new(),
            span: Span::new(1, 1, 2),
        });
        assert!(matches!(list_elem, InitElement::InitList(_)));
    }

    #[test]
    fn lambda_param_with_type() {
        use crate::ast::types::{ParamType, TypeExpr, PrimitiveType};

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
