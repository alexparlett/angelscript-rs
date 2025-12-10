//! Operator definitions for AngelScript expressions.
//!
//! Provides enums for binary, unary, and assignment operators along with
//! precedence and associativity information for the Pratt parser.

use crate::lexer::TokenKind;
use std::fmt;

/// Binary operators in AngelScript.
///
/// Organized by precedence from lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Logical OR (precedence 3)
    /// `||` or `or`
    LogicalOr,
    /// `^^` or `xor`
    LogicalXor,

    // Logical AND (precedence 4)
    /// `&&` or `and`
    LogicalAnd,

    // Bitwise OR (precedence 5)
    /// `|`
    BitwiseOr,

    // Bitwise XOR (precedence 6)
    /// `^`
    BitwiseXor,

    // Bitwise AND (precedence 7)
    /// `&`
    BitwiseAnd,

    // Equality (precedence 8)
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `is`
    Is,
    /// `!is`
    NotIs,

    // Relational (precedence 9)
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,

    // Bitwise shift (precedence 10)
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    ShiftRightUnsigned,

    // Additive (precedence 11)
    /// `+`
    Add,
    /// `-`
    Sub,

    // Multiplicative (precedence 12)
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,

    // Power (precedence 13)
    /// `**`
    Pow,
}

impl BinaryOp {
    /// Get the binding power (precedence) for this operator.
    ///
    /// Higher values bind more tightly. Returns (left_bp, right_bp).
    /// For left-associative operators: right_bp = left_bp + 1
    /// For right-associative operators: right_bp = left_bp
    pub fn binding_power(&self) -> (u8, u8) {
        use BinaryOp::*;
        match self {
            // Precedence 3 - Logical OR (left-associative)
            LogicalOr | LogicalXor => (3, 4),

            // Precedence 4 - Logical AND (left-associative)
            LogicalAnd => (5, 6),

            // Precedence 5 - Bitwise OR (left-associative)
            BitwiseOr => (7, 8),

            // Precedence 6 - Bitwise XOR (left-associative)
            BitwiseXor => (9, 10),

            // Precedence 7 - Bitwise AND (left-associative)
            BitwiseAnd => (11, 12),

            // Precedence 8 - Equality (left-associative)
            Equal | NotEqual | Is | NotIs => (13, 14),

            // Precedence 9 - Relational (left-associative)
            Less | LessEqual | Greater | GreaterEqual => (15, 16),

            // Precedence 10 - Bitwise shift (left-associative)
            ShiftLeft | ShiftRight | ShiftRightUnsigned => (17, 18),

            // Precedence 11 - Additive (left-associative)
            Add | Sub => (19, 20),

            // Precedence 12 - Multiplicative (left-associative)
            Mul | Div | Mod => (21, 22),

            // Precedence 13 - Power (right-associative)
            Pow => (24, 23), // Note: right-associative
        }
    }

    /// Try to convert a token kind to a binary operator.
    pub fn from_token(token: TokenKind) -> Option<Self> {
        use TokenKind::*;

        Some(match token {
            PipePipe | Or => BinaryOp::LogicalOr,
            CaretCaret | Xor => BinaryOp::LogicalXor,
            AmpAmp | And => BinaryOp::LogicalAnd,
            Pipe => BinaryOp::BitwiseOr,
            Caret => BinaryOp::BitwiseXor,
            Amp => BinaryOp::BitwiseAnd,
            EqualEqual => BinaryOp::Equal,
            BangEqual => BinaryOp::NotEqual,
            Is => BinaryOp::Is,
            NotIs => BinaryOp::NotIs,
            Less => BinaryOp::Less,
            LessEqual => BinaryOp::LessEqual,
            Greater => BinaryOp::Greater,
            GreaterEqual => BinaryOp::GreaterEqual,
            LessLess => BinaryOp::ShiftLeft,
            GreaterGreater => BinaryOp::ShiftRight,
            GreaterGreaterGreater => BinaryOp::ShiftRightUnsigned,
            Plus => BinaryOp::Add,
            Minus => BinaryOp::Sub,
            Star => BinaryOp::Mul,
            Slash => BinaryOp::Div,
            Percent => BinaryOp::Mod,
            StarStar => BinaryOp::Pow,
            _ => return None,
        })
    }

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

impl UnaryOp {
    /// Get the binding power for prefix operators.
    pub fn binding_power() -> u8 {
        25 // Higher than all binary operators
    }

    /// Try to convert a token kind to a unary operator.
    pub fn from_token(token: TokenKind) -> Option<Self> {
        use TokenKind::*;

        Some(match token {
            Minus => UnaryOp::Neg,
            Plus => UnaryOp::Plus,
            Bang | Not => UnaryOp::LogicalNot,
            Tilde => UnaryOp::BitwiseNot,
            PlusPlus => UnaryOp::PreInc,
            MinusMinus => UnaryOp::PreDec,
            At => UnaryOp::HandleOf,
            _ => return None,
        })
    }
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

/// Postfix operators in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostfixOp {
    /// `++` post-increment
    PostInc,
    /// `--` post-decrement
    PostDec,
}

impl PostfixOp {
    /// Get the binding power for postfix operators.
    pub fn binding_power() -> u8 {
        27 // Highest precedence
    }

    /// Try to convert a token kind to a postfix operator.
    pub fn from_token(token: TokenKind) -> Option<Self> {
        use PostfixOp::*;
        use TokenKind::*;

        Some(match token {
            PlusPlus => PostInc,
            MinusMinus => PostDec,
            _ => return None,
        })
    }
}

impl fmt::Display for PostfixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PostfixOp::*;
        let s = match self {
            PostInc => "++",
            PostDec => "--",
        };
        write!(f, "{}", s)
    }
}

/// Assignment operators in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    /// `=` simple assignment
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
    /// `&=` bitwise-and-assign
    AndAssign,
    /// `|=` bitwise-or-assign
    OrAssign,
    /// `^=` bitwise-xor-assign
    XorAssign,
    /// `<<=` shift-left-assign
    ShlAssign,
    /// `>>=` shift-right-assign
    ShrAssign,
    /// `>>>=` unsigned-shift-right-assign
    UshrAssign,
}

impl AssignOp {
    /// Get the binding power for assignment operators.
    ///
    /// Assignment is right-associative, so right_bp < left_bp.
    pub fn binding_power() -> (u8, u8) {
        (2, 1) // Lowest precedence, right-associative
    }

    /// Try to convert a token kind to an assignment operator.
    pub fn from_token(token: TokenKind) -> Option<Self> {
        use AssignOp::*;
        use TokenKind::*;

        Some(match token {
            Equal => Assign,
            PlusEqual => AddAssign,
            MinusEqual => SubAssign,
            StarEqual => MulAssign,
            SlashEqual => DivAssign,
            PercentEqual => ModAssign,
            StarStarEqual => PowAssign,
            AmpEqual => AndAssign,
            PipeEqual => OrAssign,
            CaretEqual => XorAssign,
            LessLessEqual => ShlAssign,
            GreaterGreaterEqual => ShrAssign,
            GreaterGreaterGreaterEqual => UshrAssign,
            _ => return None,
        })
    }

    /// Check if this is a simple assignment (not compound).
    pub fn is_simple(&self) -> bool {
        matches!(self, Self::Assign)
    }
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AssignOp::*;
        let s = match self {
            Assign => "=",
            AddAssign => "+=",
            SubAssign => "-=",
            MulAssign => "*=",
            DivAssign => "/=",
            ModAssign => "%=",
            PowAssign => "**=",
            AndAssign => "&=",
            OrAssign => "|=",
            XorAssign => "^=",
            ShlAssign => "<<=",
            ShrAssign => ">>=",
            UshrAssign => ">>>=",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_op_precedence() {
        // Lower precedence binds less tightly
        let (or_l, or_r) = BinaryOp::LogicalOr.binding_power();
        let (add_l, add_r) = BinaryOp::Add.binding_power();
        let (mul_l, mul_r) = BinaryOp::Mul.binding_power();

        assert!(or_l < add_l);
        assert!(add_l < mul_l);

        // Left-associative: right_bp > left_bp
        assert!(or_r > or_l); // || is left-associative
        assert!(add_r > add_l);
        assert!(mul_r > mul_l); // * is left-associative

        // Right-associative: right_bp < left_bp
        let (pow_l, pow_r) = BinaryOp::Pow.binding_power();
        assert!(pow_r < pow_l);
    }

    #[test]
    fn operator_from_token() {
        assert_eq!(BinaryOp::from_token(TokenKind::Plus), Some(BinaryOp::Add));
        assert_eq!(UnaryOp::from_token(TokenKind::Minus), Some(UnaryOp::Neg));
        assert_eq!(
            AssignOp::from_token(TokenKind::PlusEqual),
            Some(AssignOp::AddAssign)
        );
    }

    #[test]
    fn operator_display() {
        assert_eq!(format!("{}", BinaryOp::Add), "+");
        assert_eq!(format!("{}", UnaryOp::LogicalNot), "!");
        assert_eq!(format!("{}", AssignOp::AddAssign), "+=");
    }

    #[test]
    fn comparison_check() {
        assert!(BinaryOp::Equal.is_comparison());
        assert!(BinaryOp::Less.is_comparison());
        assert!(!BinaryOp::Add.is_comparison());
    }

    #[test]
    fn assignment_precedence() {
        let (assign_l, assign_r) = AssignOp::binding_power();
        let (or_l, _) = BinaryOp::LogicalOr.binding_power();

        // Assignment has lowest precedence
        assert!(assign_l < or_l);

        // Assignment is right-associative
        assert!(assign_r < assign_l);
    }

    #[test]
    fn all_binary_ops_from_token() {
        // Test all BinaryOp variants can be created from tokens
        assert_eq!(
            BinaryOp::from_token(TokenKind::PipePipe),
            Some(BinaryOp::LogicalOr)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::Or),
            Some(BinaryOp::LogicalOr)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::CaretCaret),
            Some(BinaryOp::LogicalXor)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::Xor),
            Some(BinaryOp::LogicalXor)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::AmpAmp),
            Some(BinaryOp::LogicalAnd)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::And),
            Some(BinaryOp::LogicalAnd)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::Pipe),
            Some(BinaryOp::BitwiseOr)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::Caret),
            Some(BinaryOp::BitwiseXor)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::Amp),
            Some(BinaryOp::BitwiseAnd)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::EqualEqual),
            Some(BinaryOp::Equal)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::BangEqual),
            Some(BinaryOp::NotEqual)
        );
        assert_eq!(BinaryOp::from_token(TokenKind::Is), Some(BinaryOp::Is));
        assert_eq!(
            BinaryOp::from_token(TokenKind::NotIs),
            Some(BinaryOp::NotIs)
        );
        assert_eq!(BinaryOp::from_token(TokenKind::Less), Some(BinaryOp::Less));
        assert_eq!(
            BinaryOp::from_token(TokenKind::LessEqual),
            Some(BinaryOp::LessEqual)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::Greater),
            Some(BinaryOp::Greater)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::GreaterEqual),
            Some(BinaryOp::GreaterEqual)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::LessLess),
            Some(BinaryOp::ShiftLeft)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::GreaterGreater),
            Some(BinaryOp::ShiftRight)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::GreaterGreaterGreater),
            Some(BinaryOp::ShiftRightUnsigned)
        );
        assert_eq!(BinaryOp::from_token(TokenKind::Minus), Some(BinaryOp::Sub));
        assert_eq!(BinaryOp::from_token(TokenKind::Star), Some(BinaryOp::Mul));
        assert_eq!(BinaryOp::from_token(TokenKind::Slash), Some(BinaryOp::Div));
        assert_eq!(
            BinaryOp::from_token(TokenKind::Percent),
            Some(BinaryOp::Mod)
        );
        assert_eq!(
            BinaryOp::from_token(TokenKind::StarStar),
            Some(BinaryOp::Pow)
        );

        // Test invalid token returns None
        assert_eq!(BinaryOp::from_token(TokenKind::Semicolon), None);
        assert_eq!(BinaryOp::from_token(TokenKind::IntLiteral), None);
    }

    #[test]
    fn all_binary_ops_display() {
        assert_eq!(format!("{}", BinaryOp::LogicalOr), "||");
        assert_eq!(format!("{}", BinaryOp::LogicalXor), "^^");
        assert_eq!(format!("{}", BinaryOp::LogicalAnd), "&&");
        assert_eq!(format!("{}", BinaryOp::BitwiseOr), "|");
        assert_eq!(format!("{}", BinaryOp::BitwiseXor), "^");
        assert_eq!(format!("{}", BinaryOp::BitwiseAnd), "&");
        assert_eq!(format!("{}", BinaryOp::Equal), "==");
        assert_eq!(format!("{}", BinaryOp::NotEqual), "!=");
        assert_eq!(format!("{}", BinaryOp::Is), "is");
        assert_eq!(format!("{}", BinaryOp::NotIs), "!is");
        assert_eq!(format!("{}", BinaryOp::Less), "<");
        assert_eq!(format!("{}", BinaryOp::LessEqual), "<=");
        assert_eq!(format!("{}", BinaryOp::Greater), ">");
        assert_eq!(format!("{}", BinaryOp::GreaterEqual), ">=");
        assert_eq!(format!("{}", BinaryOp::ShiftLeft), "<<");
        assert_eq!(format!("{}", BinaryOp::ShiftRight), ">>");
        assert_eq!(format!("{}", BinaryOp::ShiftRightUnsigned), ">>>");
        assert_eq!(format!("{}", BinaryOp::Sub), "-");
        assert_eq!(format!("{}", BinaryOp::Mul), "*");
        assert_eq!(format!("{}", BinaryOp::Div), "/");
        assert_eq!(format!("{}", BinaryOp::Mod), "%");
        assert_eq!(format!("{}", BinaryOp::Pow), "**");
    }

    #[test]
    fn all_binary_ops_binding_power() {
        // Test that all operators have valid binding powers
        let ops = vec![
            BinaryOp::LogicalOr,
            BinaryOp::LogicalXor,
            BinaryOp::LogicalAnd,
            BinaryOp::BitwiseOr,
            BinaryOp::BitwiseXor,
            BinaryOp::BitwiseAnd,
            BinaryOp::Equal,
            BinaryOp::NotEqual,
            BinaryOp::Is,
            BinaryOp::NotIs,
            BinaryOp::Less,
            BinaryOp::LessEqual,
            BinaryOp::Greater,
            BinaryOp::GreaterEqual,
            BinaryOp::ShiftLeft,
            BinaryOp::ShiftRight,
            BinaryOp::ShiftRightUnsigned,
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Mod,
            BinaryOp::Pow,
        ];

        for op in ops {
            let (left, right) = op.binding_power();
            assert!(left > 0, "{:?} should have positive binding power", op);
            assert!(right > 0, "{:?} should have positive binding power", op);
        }
    }

    #[test]
    fn all_comparison_ops() {
        assert!(BinaryOp::Equal.is_comparison());
        assert!(BinaryOp::NotEqual.is_comparison());
        assert!(BinaryOp::Is.is_comparison());
        assert!(BinaryOp::NotIs.is_comparison());
        assert!(BinaryOp::Less.is_comparison());
        assert!(BinaryOp::LessEqual.is_comparison());
        assert!(BinaryOp::Greater.is_comparison());
        assert!(BinaryOp::GreaterEqual.is_comparison());

        // Non-comparison ops
        assert!(!BinaryOp::LogicalOr.is_comparison());
        assert!(!BinaryOp::BitwiseAnd.is_comparison());
        assert!(!BinaryOp::Mul.is_comparison());
    }

    #[test]
    fn all_unary_ops_from_token() {
        assert_eq!(UnaryOp::from_token(TokenKind::Minus), Some(UnaryOp::Neg));
        assert_eq!(UnaryOp::from_token(TokenKind::Plus), Some(UnaryOp::Plus));
        assert_eq!(
            UnaryOp::from_token(TokenKind::Bang),
            Some(UnaryOp::LogicalNot)
        );
        assert_eq!(
            UnaryOp::from_token(TokenKind::Not),
            Some(UnaryOp::LogicalNot)
        );
        assert_eq!(
            UnaryOp::from_token(TokenKind::Tilde),
            Some(UnaryOp::BitwiseNot)
        );
        assert_eq!(
            UnaryOp::from_token(TokenKind::PlusPlus),
            Some(UnaryOp::PreInc)
        );
        assert_eq!(
            UnaryOp::from_token(TokenKind::MinusMinus),
            Some(UnaryOp::PreDec)
        );
        assert_eq!(UnaryOp::from_token(TokenKind::At), Some(UnaryOp::HandleOf));

        // Invalid token
        assert_eq!(UnaryOp::from_token(TokenKind::Semicolon), None);
    }

    #[test]
    fn all_unary_ops_display() {
        assert_eq!(format!("{}", UnaryOp::Neg), "-");
        assert_eq!(format!("{}", UnaryOp::Plus), "+");
        assert_eq!(format!("{}", UnaryOp::LogicalNot), "!");
        assert_eq!(format!("{}", UnaryOp::BitwiseNot), "~");
        assert_eq!(format!("{}", UnaryOp::PreInc), "++");
        assert_eq!(format!("{}", UnaryOp::PreDec), "--");
        assert_eq!(format!("{}", UnaryOp::HandleOf), "@");
    }

    #[test]
    fn unary_op_binding_power() {
        let bp = UnaryOp::binding_power();
        assert_eq!(bp, 25);

        // Verify it's higher than all binary ops
        let (mul_l, _) = BinaryOp::Mul.binding_power();
        assert!(bp > mul_l);
    }

    #[test]
    fn all_postfix_ops_from_token() {
        assert_eq!(
            PostfixOp::from_token(TokenKind::PlusPlus),
            Some(PostfixOp::PostInc)
        );
        assert_eq!(
            PostfixOp::from_token(TokenKind::MinusMinus),
            Some(PostfixOp::PostDec)
        );

        // Invalid token
        assert_eq!(PostfixOp::from_token(TokenKind::Semicolon), None);
    }

    #[test]
    fn all_postfix_ops_display() {
        assert_eq!(format!("{}", PostfixOp::PostInc), "++");
        assert_eq!(format!("{}", PostfixOp::PostDec), "--");
    }

    #[test]
    fn postfix_op_binding_power() {
        let bp = PostfixOp::binding_power();
        assert_eq!(bp, 27);

        // Verify it's the highest precedence
        let unary_bp = UnaryOp::binding_power();
        assert!(bp > unary_bp);
    }

    #[test]
    fn all_assign_ops_from_token() {
        assert_eq!(
            AssignOp::from_token(TokenKind::Equal),
            Some(AssignOp::Assign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::PlusEqual),
            Some(AssignOp::AddAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::MinusEqual),
            Some(AssignOp::SubAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::StarEqual),
            Some(AssignOp::MulAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::SlashEqual),
            Some(AssignOp::DivAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::PercentEqual),
            Some(AssignOp::ModAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::StarStarEqual),
            Some(AssignOp::PowAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::AmpEqual),
            Some(AssignOp::AndAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::PipeEqual),
            Some(AssignOp::OrAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::CaretEqual),
            Some(AssignOp::XorAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::LessLessEqual),
            Some(AssignOp::ShlAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::GreaterGreaterEqual),
            Some(AssignOp::ShrAssign)
        );
        assert_eq!(
            AssignOp::from_token(TokenKind::GreaterGreaterGreaterEqual),
            Some(AssignOp::UshrAssign)
        );

        // Invalid token
        assert_eq!(AssignOp::from_token(TokenKind::Semicolon), None);
    }

    #[test]
    fn all_assign_ops_display() {
        assert_eq!(format!("{}", AssignOp::Assign), "=");
        assert_eq!(format!("{}", AssignOp::AddAssign), "+=");
        assert_eq!(format!("{}", AssignOp::SubAssign), "-=");
        assert_eq!(format!("{}", AssignOp::MulAssign), "*=");
        assert_eq!(format!("{}", AssignOp::DivAssign), "/=");
        assert_eq!(format!("{}", AssignOp::ModAssign), "%=");
        assert_eq!(format!("{}", AssignOp::PowAssign), "**=");
        assert_eq!(format!("{}", AssignOp::AndAssign), "&=");
        assert_eq!(format!("{}", AssignOp::OrAssign), "|=");
        assert_eq!(format!("{}", AssignOp::XorAssign), "^=");
        assert_eq!(format!("{}", AssignOp::ShlAssign), "<<=");
        assert_eq!(format!("{}", AssignOp::ShrAssign), ">>=");
        assert_eq!(format!("{}", AssignOp::UshrAssign), ">>>=");
    }

    #[test]
    fn assign_op_is_simple() {
        assert!(AssignOp::Assign.is_simple());
        assert!(!AssignOp::AddAssign.is_simple());
        assert!(!AssignOp::SubAssign.is_simple());
        assert!(!AssignOp::MulAssign.is_simple());
        assert!(!AssignOp::DivAssign.is_simple());
        assert!(!AssignOp::ModAssign.is_simple());
        assert!(!AssignOp::PowAssign.is_simple());
        assert!(!AssignOp::AndAssign.is_simple());
        assert!(!AssignOp::OrAssign.is_simple());
        assert!(!AssignOp::XorAssign.is_simple());
        assert!(!AssignOp::ShlAssign.is_simple());
        assert!(!AssignOp::ShrAssign.is_simple());
        assert!(!AssignOp::UshrAssign.is_simple());
    }
}
