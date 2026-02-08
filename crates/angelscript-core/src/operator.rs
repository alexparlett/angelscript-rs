//! Operator enum for proc-macro attributes and behavior registration.
//!
//! This module provides the `Operator` enum used in `#[angelscript::function]`
//! attributes to specify which operator a method implements, and `ConversionEntry`
//! for storing conversion operator registrations in `TypeBehaviors`.
//!
//! # Example
//!
//! ```ignore
//! #[angelscript::function(instance, operator = Operator::Add)]
//! pub fn add(&self, other: &MyClass) -> MyClass { ... }
//! ```

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
    /// `+` addition (reverse - called on right operand)
    AddR,
    /// `-` subtraction
    Sub,
    /// `-` subtraction (reverse)
    SubR,
    /// `*` multiplication
    Mul,
    /// `*` multiplication (reverse)
    MulR,
    /// `/` division
    Div,
    /// `/` division (reverse)
    DivR,
    /// `%` modulo
    Mod,
    /// `%` modulo (reverse)
    ModR,
    /// `**` power
    Pow,
    /// `**` power (reverse)
    PowR,
    /// `&` bitwise AND
    And,
    /// `&` bitwise AND (reverse)
    AndR,
    /// `|` bitwise OR
    Or,
    /// `|` bitwise OR (reverse)
    OrR,
    /// `^` bitwise XOR
    Xor,
    /// `^` bitwise XOR (reverse)
    XorR,
    /// `<<` left shift
    Shl,
    /// `<<` left shift (reverse)
    ShlR,
    /// `>>` arithmetic right shift
    Shr,
    /// `>>` arithmetic right shift (reverse)
    ShrR,
    /// `>>>` logical right shift
    Ushr,
    /// `>>>` logical right shift (reverse)
    UshrR,

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
    /// `[]` index access (returns reference)
    Index,
    /// `[]` index getter (returns value)
    IndexGet,
    /// `[]` index setter (sets value)
    IndexSet,
    /// `()` function call
    Call,

    // === Foreach Operators ===
    /// Begin foreach iteration
    ForBegin,
    /// Check if foreach iteration is complete
    ForEnd,
    /// Advance to next foreach element
    ForNext,
    /// Get current foreach value (single value, equivalent to ForValueN(0))
    ForValue,
    /// Get foreach value at index N (multi-value iteration)
    /// The index is dynamic, allowing any number of iteration variables (up to 256)
    ForValueN(u8),

    // === Conversion ===
    /// Explicit value conversion (`opConv`)
    Conv,
    /// Implicit value conversion (`opImplConv`)
    ImplConv,
    /// Explicit handle cast (`opCast`)
    Cast,
    /// Implicit handle cast (`opImplCast`)
    ImplCast,
}

/// A conversion operator entry stored in `TypeBehaviors`.
///
/// Conversion operators need both a target type and a function hash, unlike
/// regular operators which are keyed by `Operator` alone. This struct bundles
/// all three pieces together for flat storage in a `Vec<ConversionEntry>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConversionEntry {
    /// The conversion operator kind (Conv, ImplConv, Cast, or ImplCast).
    pub op: Operator,
    /// The target type this conversion produces.
    pub target_type: crate::TypeHash,
    /// The implementing function.
    pub func_hash: crate::TypeHash,
}

impl Operator {
    /// Parse an AngelScript method name to determine the operator.
    ///
    /// Returns `None` for unrecognized method names.
    pub fn from_method_name(name: &str) -> Option<Self> {
        match name {
            // Conversion operators
            "opConv" => Some(Operator::Conv),
            "opImplConv" => Some(Operator::ImplConv),
            "opCast" => Some(Operator::Cast),
            "opImplCast" => Some(Operator::ImplCast),

            // Unary operators (prefix)
            "opNeg" => Some(Operator::Neg),
            "opCom" => Some(Operator::Com),
            "opPreInc" => Some(Operator::PreInc),
            "opPreDec" => Some(Operator::PreDec),

            // Unary operators (postfix)
            "opPostInc" => Some(Operator::PostInc),
            "opPostDec" => Some(Operator::PostDec),

            // Binary operators
            "opAdd" => Some(Operator::Add),
            "opAdd_r" => Some(Operator::AddR),
            "opSub" => Some(Operator::Sub),
            "opSub_r" => Some(Operator::SubR),
            "opMul" => Some(Operator::Mul),
            "opMul_r" => Some(Operator::MulR),
            "opDiv" => Some(Operator::Div),
            "opDiv_r" => Some(Operator::DivR),
            "opMod" => Some(Operator::Mod),
            "opMod_r" => Some(Operator::ModR),
            "opPow" => Some(Operator::Pow),
            "opPow_r" => Some(Operator::PowR),

            // Bitwise operators
            "opAnd" => Some(Operator::And),
            "opAnd_r" => Some(Operator::AndR),
            "opOr" => Some(Operator::Or),
            "opOr_r" => Some(Operator::OrR),
            "opXor" => Some(Operator::Xor),
            "opXor_r" => Some(Operator::XorR),
            "opShl" => Some(Operator::Shl),
            "opShl_r" => Some(Operator::ShlR),
            "opShr" => Some(Operator::Shr),
            "opShr_r" => Some(Operator::ShrR),
            "opUShr" => Some(Operator::Ushr),
            "opUShr_r" => Some(Operator::UshrR),

            // Comparison operators
            "opEquals" => Some(Operator::Equals),
            "opCmp" => Some(Operator::Cmp),

            // Assignment operators
            "opAssign" => Some(Operator::Assign),
            "opAddAssign" => Some(Operator::AddAssign),
            "opSubAssign" => Some(Operator::SubAssign),
            "opMulAssign" => Some(Operator::MulAssign),
            "opDivAssign" => Some(Operator::DivAssign),
            "opModAssign" => Some(Operator::ModAssign),
            "opPowAssign" => Some(Operator::PowAssign),
            "opAndAssign" => Some(Operator::AndAssign),
            "opOrAssign" => Some(Operator::OrAssign),
            "opXorAssign" => Some(Operator::XorAssign),
            "opShlAssign" => Some(Operator::ShlAssign),
            "opShrAssign" => Some(Operator::ShrAssign),
            "opUShrAssign" => Some(Operator::UshrAssign),

            // Index and call operators
            "opIndex" => Some(Operator::Index),
            "get_opIndex" => Some(Operator::IndexGet),
            "set_opIndex" => Some(Operator::IndexSet),
            "opCall" => Some(Operator::Call),

            // Foreach operators
            "opForBegin" => Some(Operator::ForBegin),
            "opForEnd" => Some(Operator::ForEnd),
            "opForNext" => Some(Operator::ForNext),
            "opForValue" => Some(Operator::ForValue),

            // Dynamic opForValue{N} - parse the index
            _ if name.starts_with("opForValue") => {
                let suffix = &name[10..]; // "opForValue".len() == 10
                suffix.parse::<u8>().ok().map(Operator::ForValueN)
            }

            _ => None,
        }
    }

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
            Operator::AddR => "opAdd_r",
            Operator::Sub => "opSub",
            Operator::SubR => "opSub_r",
            Operator::Mul => "opMul",
            Operator::MulR => "opMul_r",
            Operator::Div => "opDiv",
            Operator::DivR => "opDiv_r",
            Operator::Mod => "opMod",
            Operator::ModR => "opMod_r",
            Operator::Pow => "opPow",
            Operator::PowR => "opPow_r",
            Operator::And => "opAnd",
            Operator::AndR => "opAnd_r",
            Operator::Or => "opOr",
            Operator::OrR => "opOr_r",
            Operator::Xor => "opXor",
            Operator::XorR => "opXor_r",
            Operator::Shl => "opShl",
            Operator::ShlR => "opShl_r",
            Operator::Shr => "opShr",
            Operator::ShrR => "opShr_r",
            Operator::Ushr => "opUShr",
            Operator::UshrR => "opUShr_r",

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
            Operator::IndexGet => "get_opIndex",
            Operator::IndexSet => "set_opIndex",
            Operator::Call => "opCall",

            // Foreach
            Operator::ForBegin => "opForBegin",
            Operator::ForEnd => "opForEnd",
            Operator::ForNext => "opForNext",
            Operator::ForValue => "opForValue",
            // ForValueN is handled in Display impl since it needs dynamic formatting
            Operator::ForValueN(_) => "opForValue",

            // Conversion
            Operator::Conv => "opConv",
            Operator::ImplConv => "opImplConv",
            Operator::Cast => "opCast",
            Operator::ImplCast => "opImplCast",
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

    /// Check if this is a binary operator (including reverse variants).
    pub const fn is_binary(&self) -> bool {
        matches!(
            self,
            Operator::Add
                | Operator::AddR
                | Operator::Sub
                | Operator::SubR
                | Operator::Mul
                | Operator::MulR
                | Operator::Div
                | Operator::DivR
                | Operator::Mod
                | Operator::ModR
                | Operator::Pow
                | Operator::PowR
                | Operator::And
                | Operator::AndR
                | Operator::Or
                | Operator::OrR
                | Operator::Xor
                | Operator::XorR
                | Operator::Shl
                | Operator::ShlR
                | Operator::Shr
                | Operator::ShrR
                | Operator::Ushr
                | Operator::UshrR
        )
    }

    /// Check if this is a reverse binary operator.
    /// Reverse operators are called on the right operand when the left doesn't support the operation.
    pub const fn is_reverse(&self) -> bool {
        matches!(
            self,
            Operator::AddR
                | Operator::SubR
                | Operator::MulR
                | Operator::DivR
                | Operator::ModR
                | Operator::PowR
                | Operator::AndR
                | Operator::OrR
                | Operator::XorR
                | Operator::ShlR
                | Operator::ShrR
                | Operator::UshrR
        )
    }

    /// Check if this is an index operator.
    pub const fn is_index(&self) -> bool {
        matches!(
            self,
            Operator::Index | Operator::IndexGet | Operator::IndexSet
        )
    }

    /// Check if this is a foreach operator.
    pub const fn is_foreach(&self) -> bool {
        matches!(
            self,
            Operator::ForBegin
                | Operator::ForEnd
                | Operator::ForNext
                | Operator::ForValue
                | Operator::ForValueN(_)
        )
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::ForValueN(n) => write!(f, "opForValue{}", n),
            other => write!(f, "{}", other.method_name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_names() {
        assert_eq!(Operator::Add.method_name(), "opAdd");
        assert_eq!(Operator::AddR.method_name(), "opAdd_r");
        assert_eq!(Operator::Assign.method_name(), "opAssign");
        assert_eq!(Operator::Cmp.method_name(), "opCmp");
        assert_eq!(Operator::Index.method_name(), "opIndex");
        assert_eq!(Operator::IndexGet.method_name(), "get_opIndex");
        assert_eq!(Operator::IndexSet.method_name(), "set_opIndex");
        assert_eq!(Operator::Conv.method_name(), "opConv");
        assert_eq!(Operator::ForBegin.method_name(), "opForBegin");
        assert_eq!(Operator::ForValue.method_name(), "opForValue");
    }

    #[test]
    fn is_assignment() {
        assert!(Operator::Assign.is_assignment());
        assert!(Operator::AddAssign.is_assignment());
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

    #[test]
    fn is_binary() {
        assert!(Operator::Add.is_binary());
        assert!(Operator::AddR.is_binary());
        assert!(Operator::Sub.is_binary());
        assert!(Operator::Ushr.is_binary());
        assert!(Operator::UshrR.is_binary());
        assert!(!Operator::Neg.is_binary());
        assert!(!Operator::Index.is_binary());
    }

    #[test]
    fn is_reverse() {
        assert!(Operator::AddR.is_reverse());
        assert!(Operator::SubR.is_reverse());
        assert!(Operator::UshrR.is_reverse());
        assert!(!Operator::Add.is_reverse());
        assert!(!Operator::Sub.is_reverse());
        assert!(!Operator::Neg.is_reverse());
    }

    #[test]
    fn is_index() {
        assert!(Operator::Index.is_index());
        assert!(Operator::IndexGet.is_index());
        assert!(Operator::IndexSet.is_index());
        assert!(!Operator::Add.is_index());
        assert!(!Operator::Call.is_index());
    }

    #[test]
    fn is_foreach() {
        assert!(Operator::ForBegin.is_foreach());
        assert!(Operator::ForEnd.is_foreach());
        assert!(Operator::ForNext.is_foreach());
        assert!(Operator::ForValue.is_foreach());
        assert!(Operator::ForValueN(0).is_foreach());
        assert!(Operator::ForValueN(1).is_foreach());
        assert!(Operator::ForValueN(255).is_foreach());
        assert!(!Operator::Add.is_foreach());
        assert!(!Operator::Index.is_foreach());
    }

    #[test]
    fn for_value_n_display() {
        assert_eq!(format!("{}", Operator::ForValueN(0)), "opForValue0");
        assert_eq!(format!("{}", Operator::ForValueN(1)), "opForValue1");
        assert_eq!(format!("{}", Operator::ForValueN(42)), "opForValue42");
        assert_eq!(format!("{}", Operator::ForValueN(255)), "opForValue255");
    }
}
