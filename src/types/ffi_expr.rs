//! Owned expression types for FFI default arguments.
//!
//! This module provides `FfiExpr`, an owned expression type that can represent
//! default argument values in FFI function declarations without arena lifetimes.
//!
//! # Problem
//!
//! Default arguments in parsed function declarations are `&'ast Expr<'ast>` -
//! arena-allocated AST nodes. We need owned equivalents for `FfiFunctionDef`
//! which must be stored in `Arc<FfiRegistry>`.
//!
//! # Solution
//!
//! `FfiExpr` covers realistic FFI default argument patterns:
//! - Literals: `0`, `1.5`, `"hello"`, `true`, `null`
//! - Enum values: `Color::Red`
//! - Constructor calls: `Vec2(0, 0)`
//! - Simple unary/binary expressions: `-1`, `1 + 2`
//! - Identifiers for constants: `MAX_VALUE`
//!
//! # Example
//!
//! ```ignore
//! // Convert from parsed AST expression
//! let ffi_expr = FfiExpr::from_ast(parsed_expr)?;
//!
//! // Or construct directly
//! let default = FfiExpr::Int(42);
//! let negative = FfiExpr::Unary {
//!     op: UnaryOp::Neg,
//!     expr: Box::new(FfiExpr::Float(1.5)),
//! };
//! ```

use crate::ast::{BinaryOp, UnaryOp};

/// Owned expression for FFI default arguments.
///
/// Covers common default argument patterns without the full complexity
/// of the arena-allocated AST. Complex expressions that don't fit these
/// patterns can fall back to string storage with compile-time parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum FfiExpr {
    // === Literals ===
    /// Integer literal (e.g., `42`, `-1`)
    Int(i64),

    /// Unsigned integer literal (e.g., `42u`)
    UInt(u64),

    /// Floating-point literal (e.g., `3.14`, `1.0f`)
    /// Covers both float and double
    Float(f64),

    /// Boolean literal (`true` or `false`)
    Bool(bool),

    /// String literal (e.g., `"hello"`)
    String(String),

    /// Null literal
    Null,

    // === Compound expressions ===
    /// Enum value reference: `EnumType::Value`
    ///
    /// Example: `Color::Red`, `Direction::North`
    EnumValue {
        /// The enum type name (e.g., "Color")
        enum_name: String,
        /// The enum value name (e.g., "Red")
        value_name: String,
    },

    /// Constructor/factory call: `Type(args...)`
    ///
    /// Example: `Vec2(0, 0)`, `string("default")`
    Construct {
        /// The type being constructed
        type_name: String,
        /// Constructor arguments
        args: Vec<FfiExpr>,
    },

    /// Unary expression: `-expr`, `!expr`, `~expr`
    ///
    /// Example: `-1`, `!true`, `~0`
    Unary {
        /// The unary operator
        op: UnaryOp,
        /// The operand
        expr: Box<FfiExpr>,
    },

    /// Binary expression: `left op right`
    ///
    /// Example: `1 + 2`, `MAX - 1`
    /// Useful for simple constant math in defaults
    Binary {
        /// Left operand
        left: Box<FfiExpr>,
        /// Binary operator
        op: BinaryOp,
        /// Right operand
        right: Box<FfiExpr>,
    },

    /// Identifier reference (for constants)
    ///
    /// Example: `MAX_VALUE`, `DEFAULT_SIZE`
    Ident(String),

    /// Scoped identifier: `Namespace::Constant`
    ///
    /// Example: `Math::PI`, `Config::DEFAULT`
    ScopedIdent {
        /// Scope parts (e.g., ["Math"] for Math::PI)
        scope: Vec<String>,
        /// The identifier name
        name: String,
    },
}

impl FfiExpr {
    /// Create an integer literal expression.
    #[inline]
    pub fn int(value: i64) -> Self {
        FfiExpr::Int(value)
    }

    /// Create an unsigned integer literal expression.
    #[inline]
    pub fn uint(value: u64) -> Self {
        FfiExpr::UInt(value)
    }

    /// Create a float literal expression.
    #[inline]
    pub fn float(value: f64) -> Self {
        FfiExpr::Float(value)
    }

    /// Create a boolean literal expression.
    #[inline]
    pub fn bool(value: bool) -> Self {
        FfiExpr::Bool(value)
    }

    /// Create a string literal expression.
    #[inline]
    pub fn string(value: impl Into<String>) -> Self {
        FfiExpr::String(value.into())
    }

    /// Create a null literal expression.
    #[inline]
    pub fn null() -> Self {
        FfiExpr::Null
    }

    /// Create an enum value expression.
    #[inline]
    pub fn enum_value(enum_name: impl Into<String>, value_name: impl Into<String>) -> Self {
        FfiExpr::EnumValue {
            enum_name: enum_name.into(),
            value_name: value_name.into(),
        }
    }

    /// Create a constructor call expression.
    #[inline]
    pub fn construct(type_name: impl Into<String>, args: Vec<FfiExpr>) -> Self {
        FfiExpr::Construct {
            type_name: type_name.into(),
            args,
        }
    }

    /// Create a unary expression.
    #[inline]
    pub fn unary(op: UnaryOp, expr: FfiExpr) -> Self {
        FfiExpr::Unary {
            op,
            expr: Box::new(expr),
        }
    }

    /// Create a binary expression.
    #[inline]
    pub fn binary(left: FfiExpr, op: BinaryOp, right: FfiExpr) -> Self {
        FfiExpr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    /// Create an identifier expression.
    #[inline]
    pub fn ident(name: impl Into<String>) -> Self {
        FfiExpr::Ident(name.into())
    }

    /// Create a scoped identifier expression.
    #[inline]
    pub fn scoped_ident(scope: Vec<String>, name: impl Into<String>) -> Self {
        FfiExpr::ScopedIdent {
            scope,
            name: name.into(),
        }
    }

    /// Check if this is a simple literal (no nested expressions).
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            FfiExpr::Int(_)
                | FfiExpr::UInt(_)
                | FfiExpr::Float(_)
                | FfiExpr::Bool(_)
                | FfiExpr::String(_)
                | FfiExpr::Null
        )
    }

    /// Check if this expression is constant (can be evaluated at compile time).
    ///
    /// Note: This is a heuristic - actual constness depends on whether
    /// referenced identifiers are compile-time constants.
    pub fn is_potentially_const(&self) -> bool {
        match self {
            FfiExpr::Int(_)
            | FfiExpr::UInt(_)
            | FfiExpr::Float(_)
            | FfiExpr::Bool(_)
            | FfiExpr::String(_)
            | FfiExpr::Null => true,

            FfiExpr::EnumValue { .. } => true,

            FfiExpr::Unary { expr, .. } => expr.is_potentially_const(),

            FfiExpr::Binary { left, right, .. } => {
                left.is_potentially_const() && right.is_potentially_const()
            }

            // Identifiers might be constants, but we can't know without context
            FfiExpr::Ident(_) | FfiExpr::ScopedIdent { .. } => true,

            // Constructor calls are generally not const
            FfiExpr::Construct { .. } => false,
        }
    }

    /// Convert from an arena-allocated AST expression.
    ///
    /// Returns `None` if the expression is too complex to represent as `FfiExpr`.
    /// Complex expressions should fall back to string storage.
    pub fn from_ast(expr: &crate::ast::Expr<'_>) -> Option<Self> {
        use crate::ast::Expr;

        match expr {
            Expr::Literal(lit) => Self::from_literal(&lit.kind),

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
                    op: unary.op,
                    expr: Box::new(inner),
                })
            }

            Expr::Binary(binary) => {
                let left = Self::from_ast(binary.left)?;
                let right = Self::from_ast(binary.right)?;
                Some(FfiExpr::Binary {
                    left: Box::new(left),
                    op: binary.op,
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
                // This handles the pattern: identifier.member (though enum syntax is usually ::)
                // More commonly, enum values come through as scoped identifiers
                if let Expr::Ident(obj_ident) = member.object
                    && let crate::ast::MemberAccess::Field(field) = &member.member {
                        // This could be EnumType.Value (unusual but possible)
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

    /// Convert from a literal kind.
    fn from_literal(kind: &crate::ast::LiteralKind) -> Option<Self> {
        use crate::ast::LiteralKind;

        Some(match kind {
            LiteralKind::Int(v) => FfiExpr::Int(*v),
            LiteralKind::Float(v) => FfiExpr::Float(*v as f64),
            LiteralKind::Double(v) => FfiExpr::Float(*v),
            LiteralKind::Bool(v) => FfiExpr::Bool(*v),
            LiteralKind::String(v) => FfiExpr::String(v.clone()),
            LiteralKind::Null => FfiExpr::Null,
        })
    }
}

impl std::fmt::Display for FfiExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfiExpr::Int(v) => write!(f, "{}", v),
            FfiExpr::UInt(v) => write!(f, "{}u", v),
            FfiExpr::Float(v) => write!(f, "{}", v),
            FfiExpr::Bool(v) => write!(f, "{}", v),
            FfiExpr::String(v) => write!(f, "\"{}\"", v),
            FfiExpr::Null => write!(f, "null"),
            FfiExpr::EnumValue {
                enum_name,
                value_name,
            } => write!(f, "{}::{}", enum_name, value_name),
            FfiExpr::Construct { type_name, args } => {
                write!(f, "{}(", type_name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            FfiExpr::Unary { op, expr } => write!(f, "{}{}", op, expr),
            FfiExpr::Binary { left, op, right } => write!(f, "{} {} {}", left, op, right),
            FfiExpr::Ident(name) => write!(f, "{}", name),
            FfiExpr::ScopedIdent { scope, name } => {
                for part in scope {
                    write!(f, "{}::", part)?;
                }
                write!(f, "{}", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_constructors() {
        assert_eq!(FfiExpr::int(42), FfiExpr::Int(42));
        assert_eq!(FfiExpr::uint(42), FfiExpr::UInt(42));
        assert_eq!(FfiExpr::float(3.14), FfiExpr::Float(3.14));
        assert_eq!(FfiExpr::bool(true), FfiExpr::Bool(true));
        assert_eq!(
            FfiExpr::string("hello"),
            FfiExpr::String("hello".to_string())
        );
        assert_eq!(FfiExpr::null(), FfiExpr::Null);
    }

    #[test]
    fn compound_constructors() {
        let enum_val = FfiExpr::enum_value("Color", "Red");
        assert!(matches!(enum_val, FfiExpr::EnumValue { .. }));

        let construct = FfiExpr::construct("Vec2", vec![FfiExpr::int(0), FfiExpr::int(0)]);
        assert!(matches!(construct, FfiExpr::Construct { .. }));

        let unary = FfiExpr::unary(UnaryOp::Neg, FfiExpr::int(1));
        assert!(matches!(unary, FfiExpr::Unary { .. }));

        let binary = FfiExpr::binary(FfiExpr::int(1), BinaryOp::Add, FfiExpr::int(2));
        assert!(matches!(binary, FfiExpr::Binary { .. }));

        let ident = FfiExpr::ident("MAX_VALUE");
        assert!(matches!(ident, FfiExpr::Ident(_)));

        let scoped = FfiExpr::scoped_ident(vec!["Math".to_string()], "PI");
        assert!(matches!(scoped, FfiExpr::ScopedIdent { .. }));
    }

    #[test]
    fn is_literal() {
        assert!(FfiExpr::int(42).is_literal());
        assert!(FfiExpr::uint(42).is_literal());
        assert!(FfiExpr::float(3.14).is_literal());
        assert!(FfiExpr::bool(true).is_literal());
        assert!(FfiExpr::string("test").is_literal());
        assert!(FfiExpr::null().is_literal());

        assert!(!FfiExpr::ident("x").is_literal());
        assert!(!FfiExpr::enum_value("Color", "Red").is_literal());
        assert!(!FfiExpr::unary(UnaryOp::Neg, FfiExpr::int(1)).is_literal());
    }

    #[test]
    fn is_potentially_const() {
        // Literals are const
        assert!(FfiExpr::int(42).is_potentially_const());
        assert!(FfiExpr::null().is_potentially_const());

        // Enum values are const
        assert!(FfiExpr::enum_value("Color", "Red").is_potentially_const());

        // Identifiers might be const
        assert!(FfiExpr::ident("MAX").is_potentially_const());

        // Unary/binary on const operands are const
        assert!(FfiExpr::unary(UnaryOp::Neg, FfiExpr::int(1)).is_potentially_const());
        assert!(
            FfiExpr::binary(FfiExpr::int(1), BinaryOp::Add, FfiExpr::int(2)).is_potentially_const()
        );

        // Constructor calls are not const
        assert!(!FfiExpr::construct("Vec2", vec![]).is_potentially_const());
    }

    #[test]
    fn display_literals() {
        assert_eq!(format!("{}", FfiExpr::int(42)), "42");
        assert_eq!(format!("{}", FfiExpr::uint(42)), "42u");
        assert_eq!(format!("{}", FfiExpr::float(3.14)), "3.14");
        assert_eq!(format!("{}", FfiExpr::bool(true)), "true");
        assert_eq!(format!("{}", FfiExpr::string("hello")), "\"hello\"");
        assert_eq!(format!("{}", FfiExpr::null()), "null");
    }

    #[test]
    fn display_compound() {
        assert_eq!(
            format!("{}", FfiExpr::enum_value("Color", "Red")),
            "Color::Red"
        );

        assert_eq!(
            format!(
                "{}",
                FfiExpr::construct("Vec2", vec![FfiExpr::int(1), FfiExpr::int(2)])
            ),
            "Vec2(1, 2)"
        );

        assert_eq!(
            format!("{}", FfiExpr::unary(UnaryOp::Neg, FfiExpr::int(1))),
            "-1"
        );

        assert_eq!(
            format!(
                "{}",
                FfiExpr::binary(FfiExpr::int(1), BinaryOp::Add, FfiExpr::int(2))
            ),
            "1 + 2"
        );

        assert_eq!(format!("{}", FfiExpr::ident("MAX")), "MAX");

        assert_eq!(
            format!(
                "{}",
                FfiExpr::scoped_ident(vec!["Math".to_string()], "PI")
            ),
            "Math::PI"
        );
    }

    #[test]
    fn from_ast_literals() {
        use crate::ast::{Expr, LiteralExpr, LiteralKind};
        use crate::lexer::Span;

        let span = Span::new(1, 1, 1);

        // Int
        let int_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span,
        });
        assert_eq!(FfiExpr::from_ast(&int_expr), Some(FfiExpr::Int(42)));

        // Float
        let float_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(3.14),
            span,
        });
        let result = FfiExpr::from_ast(&float_expr);
        assert!(matches!(result, Some(FfiExpr::Float(f)) if (f - 3.14).abs() < 0.001));

        // Double
        let double_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Double(2.718),
            span,
        });
        let result = FfiExpr::from_ast(&double_expr);
        assert!(matches!(result, Some(FfiExpr::Float(f)) if (f - 2.718).abs() < 0.001));

        // Bool
        let bool_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span,
        });
        assert_eq!(FfiExpr::from_ast(&bool_expr), Some(FfiExpr::Bool(true)));

        // String
        let string_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::String("hello".to_string()),
            span,
        });
        assert_eq!(
            FfiExpr::from_ast(&string_expr),
            Some(FfiExpr::String("hello".to_string()))
        );

        // Null
        let null_expr = Expr::Literal(LiteralExpr {
            kind: LiteralKind::Null,
            span,
        });
        assert_eq!(FfiExpr::from_ast(&null_expr), Some(FfiExpr::Null));
    }

    #[test]
    fn from_ast_identifier() {
        use crate::ast::{Expr, Ident, IdentExpr};
        use crate::lexer::Span;

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
        use crate::ast::{Expr, Ident, IdentExpr, Scope};
        use crate::lexer::Span;

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
        use crate::ast::{Expr, LiteralExpr, LiteralKind, UnaryExpr};
        use crate::lexer::Span;

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
            Some(FfiExpr::Unary { op: UnaryOp::Neg, .. })
        ));
    }

    #[test]
    fn from_ast_binary() {
        use bumpalo::Bump;
        use crate::ast::{BinaryExpr, Expr, LiteralExpr, LiteralKind};
        use crate::lexer::Span;

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
            Some(FfiExpr::Binary { op: BinaryOp::Add, .. })
        ));
    }

    #[test]
    fn from_ast_call_constructor() {
        use bumpalo::Bump;
        use crate::ast::{Argument, CallExpr, Expr, Ident, IdentExpr, LiteralExpr, LiteralKind};
        use crate::lexer::Span;

        let arena = Bump::new();
        let span = Span::new(1, 1, 1);

        let arg_value = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(0),
            span,
        }));

        let args = arena.alloc_slice_copy(&[Argument {
            name: None,
            value: arg_value,
            span,
        }]);

        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("Vec2", span),
            type_args: &[],
            span,
        }));

        let call_expr = Expr::Call(arena.alloc(CallExpr { callee, args, span }));

        let result = FfiExpr::from_ast(&call_expr);
        assert!(matches!(
            result,
            Some(FfiExpr::Construct { type_name, args })
            if type_name == "Vec2" && args.len() == 1
        ));
    }

    #[test]
    fn from_ast_paren() {
        use bumpalo::Bump;
        use crate::ast::{Expr, LiteralExpr, LiteralKind, ParenExpr};
        use crate::lexer::Span;

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
        use crate::ast::{AssignExpr, AssignOp, Expr, Ident, IdentExpr};
        use crate::lexer::Span;

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

    #[test]
    fn equality() {
        let a = FfiExpr::int(42);
        let b = FfiExpr::int(42);
        let c = FfiExpr::int(43);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn clone() {
        let original = FfiExpr::construct(
            "Vec2",
            vec![
                FfiExpr::unary(UnaryOp::Neg, FfiExpr::int(1)),
                FfiExpr::binary(FfiExpr::int(2), BinaryOp::Mul, FfiExpr::int(3)),
            ],
        );

        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn debug() {
        let expr = FfiExpr::enum_value("Color", "Red");
        let debug = format!("{:?}", expr);
        assert!(debug.contains("EnumValue"));
        assert!(debug.contains("Color"));
        assert!(debug.contains("Red"));
    }
}
