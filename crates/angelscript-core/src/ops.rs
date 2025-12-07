//! Operator definitions for AngelScript expressions.
//!
//! Provides enums for binary and unary operators used in FFI expressions
//! and other core type definitions.

use std::fmt;

/// Binary operators in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Logical operators
    /// `||` or `or`
    LogicalOr,
    /// `^^` or `xor`
    LogicalXor,
    /// `&&` or `and`
    LogicalAnd,

    // Bitwise operators
    /// `|`
    BitwiseOr,
    /// `^`
    BitwiseXor,
    /// `&`
    BitwiseAnd,

    // Equality operators
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `is`
    Is,
    /// `!is`
    NotIs,

    // Relational operators
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,

    // Bitwise shift operators
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    ShiftRightUnsigned,

    // Arithmetic operators
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `**`
    Pow,
}

impl BinaryOp {
    /// Check if this operator is comparison-related.
    pub fn is_comparison(&self) -> bool {
        use BinaryOp::*;
        matches!(
            self,
            Equal | NotEqual | Is | NotIs | Less | LessEqual | Greater | GreaterEqual
        )
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BinaryOp::*;
        let s = match self {
            LogicalOr => "||",
            LogicalXor => "^^",
            LogicalAnd => "&&",
            BitwiseOr => "|",
            BitwiseXor => "^",
            BitwiseAnd => "&",
            Equal => "==",
            NotEqual => "!=",
            Is => "is",
            NotIs => "!is",
            Less => "<",
            LessEqual => "<=",
            Greater => ">",
            GreaterEqual => ">=",
            ShiftLeft => "<<",
            ShiftRight => ">>",
            ShiftRightUnsigned => ">>>",
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Mod => "%",
            Pow => "**",
        };
        write!(f, "{}", s)
    }
}

/// Unary prefix operators in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `-` negation
    Neg,
    /// `+` plus (unary)
    Plus,
    /// `!` or `not` logical NOT
    LogicalNot,
    /// `~` bitwise NOT
    BitwiseNot,
    /// `++` pre-increment
    PreInc,
    /// `--` pre-decrement
    PreDec,
    /// `@` handle-of
    HandleOf,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use UnaryOp::*;
        let s = match self {
            Neg => "-",
            Plus => "+",
            LogicalNot => "!",
            BitwiseNot => "~",
            PreInc => "++",
            PreDec => "--",
            HandleOf => "@",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_op_display() {
        assert_eq!(format!("{}", BinaryOp::Add), "+");
        assert_eq!(format!("{}", BinaryOp::LogicalOr), "||");
        assert_eq!(format!("{}", BinaryOp::Equal), "==");
        assert_eq!(format!("{}", BinaryOp::Pow), "**");
    }

    #[test]
    fn unary_op_display() {
        assert_eq!(format!("{}", UnaryOp::Neg), "-");
        assert_eq!(format!("{}", UnaryOp::LogicalNot), "!");
        assert_eq!(format!("{}", UnaryOp::HandleOf), "@");
    }

    #[test]
    fn binary_op_is_comparison() {
        assert!(BinaryOp::Equal.is_comparison());
        assert!(BinaryOp::Less.is_comparison());
        assert!(!BinaryOp::Add.is_comparison());
        assert!(!BinaryOp::LogicalAnd.is_comparison());
    }
}
