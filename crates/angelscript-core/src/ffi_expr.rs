//! Owned expression types for FFI default arguments.
//!
//! This module provides `FfiExpr`, an owned expression type that can represent
//! default argument values in FFI function declarations without arena lifetimes.
//!
//! # Problem
//!
//! Default arguments in parsed function declarations are arena-allocated AST nodes.
//! We need owned equivalents for FFI function definitions which must be stored
//! without lifetime dependencies.
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
//! ```
//! use angelscript_core::{FfiExpr, UnaryOp, BinaryOp};
//!
//! let default = FfiExpr::Int(42);
//! let negative = FfiExpr::Unary {
//!     op: UnaryOp::Neg,
//!     expr: Box::new(FfiExpr::Float(1.5)),
//! };
//! let sum = FfiExpr::binary(FfiExpr::int(1), BinaryOp::Add, FfiExpr::int(2));
//! ```

use crate::ops::{BinaryOp, UnaryOp};

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
