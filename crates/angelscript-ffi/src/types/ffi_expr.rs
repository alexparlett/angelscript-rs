//! FFI expression types and AST conversion.
//!
//! This module re-exports `FfiExpr` from `angelscript-core` and provides
//! the `from_ast` conversion method for converting arena-allocated AST
//! expressions to owned `FfiExpr` values.

// Re-export FfiExpr from core
pub use angelscript_core::FfiExpr;

use angelscript_parser::ast::{BinaryOp, UnaryOp};

/// Extension trait for FfiExpr that provides AST conversion.
pub trait FfiExprExt {
    /// Convert from an arena-allocated AST expression.
    ///
    /// Returns `None` if the expression is too complex to represent as `FfiExpr`.
    /// Complex expressions should fall back to string storage.
    fn from_ast(expr: &angelscript_parser::ast::Expr<'_>) -> Option<FfiExpr>;
}

impl FfiExprExt for FfiExpr {
    fn from_ast(expr: &angelscript_parser::ast::Expr<'_>) -> Option<FfiExpr> {
        use angelscript_parser::ast::Expr;

        match expr {
            Expr::Literal(lit) => from_literal(&lit.kind),

            Expr::Ident(ident_expr) => {
                let name = ident_expr.ident.name.to_string();

                // Check for scoped identifier (e.g., Namespace::Value)
                if let Some(scope) = &ident_expr.scope {
                    let scope_parts: Vec<String> =
                        scope.segments.iter().map(|p| p.name.to_string()).collect();
                    Some(FfiExpr::ScopedIdent {
                        scope: scope_parts,
                        name,
                    })
                } else {
                    Some(FfiExpr::Ident(name))
                }
            }

            Expr::Unary(unary) => {
                let inner = Self::from_ast(unary.operand)?;
                Some(FfiExpr::Unary {
                    op: convert_unary_op(unary.op),
                    expr: Box::new(inner),
                })
            }

            Expr::Binary(binary) => {
                let left = Self::from_ast(binary.left)?;
                let right = Self::from_ast(binary.right)?;
                Some(FfiExpr::Binary {
                    left: Box::new(left),
                    op: convert_binary_op(binary.op),
                    right: Box::new(right),
                })
            }

            Expr::Call(call) => {
                // Check if this is a constructor call (identifier followed by args)
                if let Expr::Ident(ident_expr) = call.callee {
                    // Handle scoped constructor: Namespace::Type(args)
                    let type_name = if let Some(scope) = &ident_expr.scope {
                        let mut parts: Vec<&str> =
                            scope.segments.iter().map(|p| p.name).collect();
                        parts.push(ident_expr.ident.name);
                        parts.join("::")
                    } else {
                        ident_expr.ident.name.to_string()
                    };

                    let args: Option<Vec<FfiExpr>> = call
                        .args
                        .iter()
                        .map(|arg| Self::from_ast(arg.value))
                        .collect();

                    Some(FfiExpr::Construct {
                        type_name,
                        args: args?,
                    })
                } else {
                    None // Complex callee not supported
                }
            }

            Expr::Member(member) => {
                // Check for enum value: EnumType::Value or just Value accessed as member
                if let Expr::Ident(obj_ident) = member.object
                    && let angelscript_parser::ast::MemberAccess::Field(field) = &member.member {
                        return Some(FfiExpr::EnumValue {
                            enum_name: obj_ident.ident.name.to_string(),
                            value_name: field.name.to_string(),
                        });
                    }
                None
            }

            Expr::Paren(paren) => {
                // Unwrap parenthesized expressions
                Self::from_ast(paren.expr)
            }

            // Not supported for FFI defaults
            Expr::Assign(_)
            | Expr::Ternary(_)
            | Expr::Index(_)
            | Expr::Postfix(_)
            | Expr::Cast(_)
            | Expr::Lambda(_)
            | Expr::InitList(_) => None,
        }
    }
}

/// Convert from a literal kind.
fn from_literal(kind: &angelscript_parser::ast::LiteralKind) -> Option<FfiExpr> {
    use angelscript_parser::ast::LiteralKind;

    Some(match kind {
        LiteralKind::Int(v) => FfiExpr::Int(*v),
        LiteralKind::Float(v) => FfiExpr::Float(*v as f64),
        LiteralKind::Double(v) => FfiExpr::Float(*v),
        LiteralKind::Bool(v) => FfiExpr::Bool(*v),
        LiteralKind::String(v) => FfiExpr::String(v.clone()),
        LiteralKind::Null => FfiExpr::Null,
    })
}

/// Convert AST UnaryOp to core UnaryOp.
fn convert_unary_op(op: UnaryOp) -> angelscript_core::UnaryOp {
    use angelscript_core::UnaryOp as CoreOp;
    match op {
        UnaryOp::Neg => CoreOp::Neg,
        UnaryOp::Plus => CoreOp::Plus,
        UnaryOp::LogicalNot => CoreOp::LogicalNot,
        UnaryOp::BitwiseNot => CoreOp::BitwiseNot,
        UnaryOp::PreInc => CoreOp::PreInc,
        UnaryOp::PreDec => CoreOp::PreDec,
        UnaryOp::HandleOf => CoreOp::HandleOf,
    }
}

/// Convert AST BinaryOp to core BinaryOp.
fn convert_binary_op(op: BinaryOp) -> angelscript_core::BinaryOp {
    use angelscript_core::BinaryOp as CoreOp;
    match op {
        BinaryOp::LogicalOr => CoreOp::LogicalOr,
        BinaryOp::LogicalXor => CoreOp::LogicalXor,
        BinaryOp::LogicalAnd => CoreOp::LogicalAnd,
        BinaryOp::BitwiseOr => CoreOp::BitwiseOr,
        BinaryOp::BitwiseXor => CoreOp::BitwiseXor,
        BinaryOp::BitwiseAnd => CoreOp::BitwiseAnd,
        BinaryOp::Equal => CoreOp::Equal,
        BinaryOp::NotEqual => CoreOp::NotEqual,
        BinaryOp::Is => CoreOp::Is,
        BinaryOp::NotIs => CoreOp::NotIs,
        BinaryOp::Less => CoreOp::Less,
        BinaryOp::LessEqual => CoreOp::LessEqual,
        BinaryOp::Greater => CoreOp::Greater,
        BinaryOp::GreaterEqual => CoreOp::GreaterEqual,
        BinaryOp::ShiftLeft => CoreOp::ShiftLeft,
        BinaryOp::ShiftRight => CoreOp::ShiftRight,
        BinaryOp::ShiftRightUnsigned => CoreOp::ShiftRightUnsigned,
        BinaryOp::Add => CoreOp::Add,
        BinaryOp::Sub => CoreOp::Sub,
        BinaryOp::Mul => CoreOp::Mul,
        BinaryOp::Div => CoreOp::Div,
        BinaryOp::Mod => CoreOp::Mod,
        BinaryOp::Pow => CoreOp::Pow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ast_literals() {
        use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind};
        use angelscript_parser::lexer::Span;

        let span = Span::new(1, 1, 1);

        // Int
        let int_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span,
        });
        assert_eq!(FfiExpr::from_ast(&int_expr), Some(FfiExpr::Int(42)));

        // Bool
        let bool_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span,
        });
        assert_eq!(FfiExpr::from_ast(&bool_expr), Some(FfiExpr::Bool(true)));

        // Null
        let null_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Null,
            span,
        });
        assert_eq!(FfiExpr::from_ast(&null_expr), Some(FfiExpr::Null));
    }

    #[test]
    fn from_ast_identifier() {
        use angelscript_parser::ast::{Expr, Ident, IdentExpr};
        use angelscript_parser::lexer::Span;

        let span = Span::new(1, 1, 1);

        let ident_expr = Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("MAX_VALUE", span),
            type_args: &[],
            span,
        });

        assert_eq!(
            FfiExpr::from_ast(&ident_expr),
            Some(FfiExpr::Ident("MAX_VALUE".to_string()))
        );
    }

    #[test]
    fn from_ast_scoped_identifier() {
        use bumpalo::Bump;
        use angelscript_parser::ast::{Expr, Ident, IdentExpr, Scope};
        use angelscript_parser::lexer::Span;

        let arena = Bump::new();
        let span = Span::new(1, 1, 1);

        let segments = arena.alloc_slice_copy(&[Ident::new("Math", span)]);
        let scope = Scope {
            is_absolute: false,
            segments,
            span,
        };

        let ident_expr = Expr::Ident(IdentExpr {
            scope: Some(scope),
            ident: Ident::new("PI", span),
            type_args: &[],
            span,
        });

        let result = FfiExpr::from_ast(&ident_expr);
        assert!(matches!(
            result,
            Some(FfiExpr::ScopedIdent { scope, name })
            if scope == vec!["Math".to_string()] && name == "PI"
        ));
    }

    #[test]
    fn from_ast_unary() {
        use bumpalo::Bump;
        use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind, UnaryExpr, UnaryOp};
        use angelscript_parser::lexer::Span;

        let arena = Bump::new();
        let span = Span::new(1, 1, 1);

        let operand = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span,
        }));

        let unary_expr = Expr::Unary(arena.alloc(UnaryExpr {
            op: UnaryOp::Neg,
            operand,
            span,
        }));

        let result = FfiExpr::from_ast(&unary_expr);
        assert!(matches!(
            result,
            Some(FfiExpr::Unary { op: angelscript_core::UnaryOp::Neg, .. })
        ));
    }

    #[test]
    fn from_ast_binary() {
        use bumpalo::Bump;
        use angelscript_parser::ast::{BinaryExpr, BinaryOp, Expr, LiteralExpr, LiteralKind};
        use angelscript_parser::lexer::Span;

        let arena = Bump::new();
        let span = Span::new(1, 1, 1);

        let left = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span,
        }));
        let right = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span,
        }));

        let binary_expr = Expr::Binary(arena.alloc(BinaryExpr {
            left,
            op: BinaryOp::Add,
            right,
            span,
        }));

        let result = FfiExpr::from_ast(&binary_expr);
        assert!(matches!(
            result,
            Some(FfiExpr::Binary { op: angelscript_core::BinaryOp::Add, .. })
        ));
    }

    #[test]
    fn from_ast_paren() {
        use bumpalo::Bump;
        use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind, ParenExpr};
        use angelscript_parser::lexer::Span;

        let arena = Bump::new();
        let span = Span::new(1, 1, 1);

        let inner = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span,
        }));

        let paren_expr = Expr::Paren(arena.alloc(ParenExpr { expr: inner, span }));

        // Should unwrap the parentheses
        assert_eq!(FfiExpr::from_ast(&paren_expr), Some(FfiExpr::Int(42)));
    }

    #[test]
    fn from_ast_unsupported() {
        use bumpalo::Bump;
        use angelscript_parser::ast::{AssignExpr, AssignOp, Expr, Ident, IdentExpr};
        use angelscript_parser::lexer::Span;

        let arena = Bump::new();
        let span = Span::new(1, 1, 1);

        let target = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("x", span),
            type_args: &[],
            span,
        }));
        let value = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("y", span),
            type_args: &[],
            span,
        }));

        // Assignment expressions are not supported for default args
        let assign_expr = Expr::Assign(arena.alloc(AssignExpr {
            target,
            op: AssignOp::Assign,
            value,
            span,
        }));

        assert_eq!(FfiExpr::from_ast(&assign_expr), None);
    }
}
