//! Operator enum for proc-macro attributes.
//!
//! This module provides the `Operator` enum used in `#[angelscript::function]`
//! attributes to specify which operator a method implements.
//!
//! # Example
//!
//! ```ignore
//! #[angelscript::function(instance, operator = Operator::Add)]
//! pub fn add(&self, other: &MyClass) -> MyClass { ... }
//! ```
//!
//! Note: This is distinct from `OperatorBehavior` in `type_def.rs`, which
//! includes target types for conversion operators and is used in the registry.

use std::fmt;

/// Operator kinds for method registration via proc-macros.
///
/// Used in `#[angelscript::function(operator = ...)]` to specify
/// which AngelScript operator a method implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    // === Assignment Operators ===
    /// `=` assignment
    Assign,
    /// `+=` add-assign
    AddAssign,
    /// `-=` subtract-assign
    SubAssign,
    /// `*=` multiply-assign
    MulAssign,
    /// `/=` divide-assign
    DivAssign,
    /// `%=` modulo-assign
    ModAssign,
    /// `**=` power-assign
    PowAssign,
    /// `&=` bitwise AND-assign
    AndAssign,
    /// `|=` bitwise OR-assign
    OrAssign,
    /// `^=` bitwise XOR-assign
    XorAssign,
    /// `<<=` left shift-assign
    ShlAssign,
    /// `>>=` arithmetic right shift-assign
    ShrAssign,
    /// `>>>=` logical right shift-assign
    UshrAssign,

    // === Binary Operators ===
    /// `+` addition
    Add,
    /// `-` subtraction
    Sub,
    /// `*` multiplication
    Mul,
    /// `/` division
    Div,
    /// `%` modulo
    Mod,
    /// `**` power
    Pow,
    /// `&` bitwise AND
    And,
    /// `|` bitwise OR
    Or,
    /// `^` bitwise XOR
    Xor,
    /// `<<` left shift
    Shl,
    /// `>>` arithmetic right shift
    Shr,
    /// `>>>` logical right shift
    Ushr,

    // === Comparison Operators ===
    /// `opCmp` - returns int for ordering
    Cmp,
    /// `opEquals` - returns bool for equality
    Equals,

    // === Unary Operators ===
    /// `-` unary negation
    Neg,
    /// `~` bitwise complement
    Com,
    /// `++x` pre-increment
    PreInc,
    /// `--x` pre-decrement
    PreDec,
    /// `x++` post-increment
    PostInc,
    /// `x--` post-decrement
    PostDec,

    // === Index and Call ===
    /// `[]` index access
    Index,
    /// `()` function call
    Call,

    // === Conversion ===
    /// Explicit value conversion (`opConv`)
    Conv,
    /// Implicit value conversion (`opImplConv`)
    ImplConv,
    /// Explicit handle cast (`opCast`)
    Cast,
    /// Implicit handle cast (`opImplCast`)
    ImplCast,

    // === Handle Assignment ===
    /// `@=` handle assignment for generic handle types
    HndlAssign,
}

impl Operator {
    /// Get the AngelScript method name for this operator.
    pub const fn method_name(&self) -> &'static str {
        match self {
            // Assignment
            Operator::Assign => "opAssign",
            Operator::AddAssign => "opAddAssign",
            Operator::SubAssign => "opSubAssign",
            Operator::MulAssign => "opMulAssign",
            Operator::DivAssign => "opDivAssign",
            Operator::ModAssign => "opModAssign",
            Operator::PowAssign => "opPowAssign",
            Operator::AndAssign => "opAndAssign",
            Operator::OrAssign => "opOrAssign",
            Operator::XorAssign => "opXorAssign",
            Operator::ShlAssign => "opShlAssign",
            Operator::ShrAssign => "opShrAssign",
            Operator::UshrAssign => "opUShrAssign",

            // Binary
            Operator::Add => "opAdd",
            Operator::Sub => "opSub",
            Operator::Mul => "opMul",
            Operator::Div => "opDiv",
            Operator::Mod => "opMod",
            Operator::Pow => "opPow",
            Operator::And => "opAnd",
            Operator::Or => "opOr",
            Operator::Xor => "opXor",
            Operator::Shl => "opShl",
            Operator::Shr => "opShr",
            Operator::Ushr => "opUShr",

            // Comparison
            Operator::Cmp => "opCmp",
            Operator::Equals => "opEquals",

            // Unary
            Operator::Neg => "opNeg",
            Operator::Com => "opCom",
            Operator::PreInc => "opPreInc",
            Operator::PreDec => "opPreDec",
            Operator::PostInc => "opPostInc",
            Operator::PostDec => "opPostDec",

            // Index and Call
            Operator::Index => "opIndex",
            Operator::Call => "opCall",

            // Conversion
            Operator::Conv => "opConv",
            Operator::ImplConv => "opImplConv",
            Operator::Cast => "opCast",
            Operator::ImplCast => "opImplCast",

            // Handle
            Operator::HndlAssign => "opHndlAssign",
        }
    }

    /// Check if this is an assignment operator.
    pub const fn is_assignment(&self) -> bool {
        matches!(
            self,
            Operator::Assign
                | Operator::AddAssign
                | Operator::SubAssign
                | Operator::MulAssign
                | Operator::DivAssign
                | Operator::ModAssign
                | Operator::PowAssign
                | Operator::AndAssign
                | Operator::OrAssign
                | Operator::XorAssign
                | Operator::ShlAssign
                | Operator::ShrAssign
                | Operator::UshrAssign
                | Operator::HndlAssign
        )
    }

    /// Check if this is a comparison operator.
    pub const fn is_comparison(&self) -> bool {
        matches!(self, Operator::Cmp | Operator::Equals)
    }

    /// Check if this is a unary operator.
    pub const fn is_unary(&self) -> bool {
        matches!(
            self,
            Operator::Neg
                | Operator::Com
                | Operator::PreInc
                | Operator::PreDec
                | Operator::PostInc
                | Operator::PostDec
        )
    }

    /// Check if this is a conversion operator.
    pub const fn is_conversion(&self) -> bool {
        matches!(
            self,
            Operator::Conv | Operator::ImplConv | Operator::Cast | Operator::ImplCast
        )
    }

    /// Check if this is an implicit conversion/cast.
    pub const fn is_implicit(&self) -> bool {
        matches!(self, Operator::ImplConv | Operator::ImplCast)
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.method_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names() {
        assert_eq!(Operator::Add.method_name(), "opAdd");
        assert_eq!(Operator::Assign.method_name(), "opAssign");
        assert_eq!(Operator::Cmp.method_name(), "opCmp");
        assert_eq!(Operator::Index.method_name(), "opIndex");
        assert_eq!(Operator::Conv.method_name(), "opConv");
    }

    #[test]
    fn is_assignment() {
        assert!(Operator::Assign.is_assignment());
        assert!(Operator::AddAssign.is_assignment());
        assert!(Operator::HndlAssign.is_assignment());
        assert!(!Operator::Add.is_assignment());
        assert!(!Operator::Cmp.is_assignment());
    }

    #[test]
    fn is_comparison() {
        assert!(Operator::Cmp.is_comparison());
        assert!(Operator::Equals.is_comparison());
        assert!(!Operator::Add.is_comparison());
    }

    #[test]
    fn is_unary() {
        assert!(Operator::Neg.is_unary());
        assert!(Operator::PreInc.is_unary());
        assert!(Operator::PostDec.is_unary());
        assert!(!Operator::Add.is_unary());
    }

    #[test]
    fn is_conversion() {
        assert!(Operator::Conv.is_conversion());
        assert!(Operator::ImplConv.is_conversion());
        assert!(Operator::Cast.is_conversion());
        assert!(Operator::ImplCast.is_conversion());
        assert!(!Operator::Add.is_conversion());
    }

    #[test]
    fn is_implicit() {
        assert!(Operator::ImplConv.is_implicit());
        assert!(Operator::ImplCast.is_implicit());
        assert!(!Operator::Conv.is_implicit());
        assert!(!Operator::Cast.is_implicit());
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Operator::Add), "opAdd");
        assert_eq!(format!("{}", Operator::Equals), "opEquals");
    }
}
