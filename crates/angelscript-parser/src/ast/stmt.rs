//! Statement AST nodes for AngelScript.
//!
//! Provides nodes for all statement types including:
//! - Expression statements
//! - Variable declarations
//! - Control flow (if, while, for, switch)
//! - Loops (while, do-while, for, foreach)
//! - Jump statements (return, break, continue)
//! - Exception handling (try-catch)
//! - Blocks

use crate::ast::Ident;
use crate::ast::expr::Expr;
use crate::ast::types::TypeExpr;
use angelscript_core::Span;

/// A statement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stmt<'ast> {
    /// Expression statement (expr;)
    Expr(ExprStmt<'ast>),
    /// Variable declaration
    VarDecl(VarDeclStmt<'ast>),
    /// Return statement
    Return(ReturnStmt<'ast>),
    /// Break statement
    Break(BreakStmt),
    /// Continue statement
    Continue(ContinueStmt),
    /// Block statement
    Block(Block<'ast>),
    /// If statement
    If(&'ast IfStmt<'ast>),
    /// While loop
    While(&'ast WhileStmt<'ast>),
    /// Do-while loop
    DoWhile(&'ast DoWhileStmt<'ast>),
    /// For loop
    For(&'ast ForStmt<'ast>),
    /// Foreach loop
    Foreach(&'ast ForeachStmt<'ast>),
    /// Switch statement
    Switch(&'ast SwitchStmt<'ast>),
    /// Try-catch statement
    TryCatch(&'ast TryCatchStmt<'ast>),
}

impl<'ast> Stmt<'ast> {
    /// Get the span of this statement.
    pub fn span(&self) -> Span {
        match self {
            Self::Expr(s) => s.span,
            Self::VarDecl(s) => s.span,
            Self::Return(s) => s.span,
            Self::Break(s) => s.span,
            Self::Continue(s) => s.span,
            Self::Block(s) => s.span,
            Self::If(s) => s.span,
            Self::While(s) => s.span,
            Self::DoWhile(s) => s.span,
            Self::For(s) => s.span,
            Self::Foreach(s) => s.span,
            Self::Switch(s) => s.span,
            Self::TryCatch(s) => s.span,
        }
    }
}

/// An expression statement (expression followed by semicolon).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExprStmt<'ast> {
    /// The expression (can be None for empty statement `;`)
    pub expr: Option<&'ast Expr<'ast>>,
    /// Source location
    pub span: Span,
}

/// A variable declaration statement.
///
/// Examples:
/// - `int x;`
/// - `int x = 5;`
/// - `int x = 5, y = 10;`
/// - `MyClass@ obj = MyClass();`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VarDeclStmt<'ast> {
    /// The type of the variable(s)
    pub ty: TypeExpr<'ast>,
    /// Variable declarations (can be multiple)
    pub vars: &'ast [VarDeclarator<'ast>],
    /// Source location
    pub span: Span,
}

/// A single variable declarator within a variable declaration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VarDeclarator<'ast> {
    /// Variable name
    pub name: Ident<'ast>,
    /// Optional initializer
    pub init: Option<&'ast Expr<'ast>>,
    /// Source location
    pub span: Span,
}

/// A return statement.
///
/// Examples:
/// - `return;`
/// - `return expr;`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReturnStmt<'ast> {
    /// Optional return value
    pub value: Option<&'ast Expr<'ast>>,
    /// Source location
    pub span: Span,
}

/// A break statement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakStmt {
    /// Source location
    pub span: Span,
}

/// A continue statement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContinueStmt {
    /// Source location
    pub span: Span,
}

/// A block of statements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block<'ast> {
    /// Statements in the block
    pub stmts: &'ast [Stmt<'ast>],
    /// Source location
    pub span: Span,
}

/// An if statement.
///
/// Examples:
/// - `if (condition) statement`
/// - `if (condition) statement else statement`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IfStmt<'ast> {
    /// Condition
    pub condition: &'ast Expr<'ast>,
    /// Then branch
    pub then_stmt: &'ast Stmt<'ast>,
    /// Optional else branch
    pub else_stmt: Option<&'ast Stmt<'ast>>,
    /// Source location
    pub span: Span,
}

/// A while loop.
///
/// Example: `while (condition) statement`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhileStmt<'ast> {
    /// Condition
    pub condition: &'ast Expr<'ast>,
    /// Body
    pub body: &'ast Stmt<'ast>,
    /// Source location
    pub span: Span,
}

/// A do-while loop.
///
/// Example: `do statement while (condition);`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoWhileStmt<'ast> {
    /// Body
    pub body: &'ast Stmt<'ast>,
    /// Condition
    pub condition: &'ast Expr<'ast>,
    /// Source location
    pub span: Span,
}

/// A for loop.
///
/// Example: `for (init; condition; update) statement`
///
/// The init can be either a variable declaration or an expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForStmt<'ast> {
    /// Initializer (variable declaration or expression)
    pub init: Option<ForInit<'ast>>,
    /// Condition
    pub condition: Option<&'ast Expr<'ast>>,
    /// Update expressions
    pub update: &'ast [&'ast Expr<'ast>],
    /// Body
    pub body: &'ast Stmt<'ast>,
    /// Source location
    pub span: Span,
}

/// The initializer in a for loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForInit<'ast> {
    /// Variable declaration
    VarDecl(VarDeclStmt<'ast>),
    /// Expression
    Expr(&'ast Expr<'ast>),
}

/// A foreach loop.
///
/// Example: `foreach (Type var : expr) statement`
///
/// AngelScript also supports multiple iteration variables:
/// `foreach (Type1 var1, Type2 var2 : expr) statement`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForeachStmt<'ast> {
    /// Iteration variables
    pub vars: &'ast [ForeachVar<'ast>],
    /// Expression to iterate over
    pub expr: &'ast Expr<'ast>,
    /// Body
    pub body: &'ast Stmt<'ast>,
    /// Source location
    pub span: Span,
}

/// A foreach iteration variable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForeachVar<'ast> {
    /// Variable type
    pub ty: TypeExpr<'ast>,
    /// Variable name
    pub name: Ident<'ast>,
    /// Source location
    pub span: Span,
}

/// A switch statement.
///
/// Example:
/// ```as
/// switch (expr) {
///     case 1:
///     case 2:
///         statement;
///         break;
///     default:
///         statement;
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchStmt<'ast> {
    /// Expression to switch on
    pub expr: &'ast Expr<'ast>,
    /// Cases
    pub cases: &'ast [SwitchCase<'ast>],
    /// Source location
    pub span: Span,
}

/// A switch case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchCase<'ast> {
    /// Case values (empty for default)
    pub values: &'ast [&'ast Expr<'ast>],
    /// Statements
    pub stmts: &'ast [Stmt<'ast>],
    /// Source location
    pub span: Span,
}

impl<'ast> SwitchCase<'ast> {
    /// Check if this is the default case.
    pub fn is_default(&self) -> bool {
        self.values.is_empty()
    }
}

/// A try-catch statement.
///
/// Example:
/// ```as
/// try {
///     statement;
/// } catch {
///     statement;
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TryCatchStmt<'ast> {
    /// Try block
    pub try_block: Block<'ast>,
    /// Catch block
    pub catch_block: Block<'ast>,
    /// Source location
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stmt_span() {
        let stmt = Stmt::Break(BreakStmt {
            span: Span::new(1, 0 + 1, 6 - 0),
        });
        assert_eq!(stmt.span(), Span::new(1, 0 + 1, 6 - 0));
    }

    #[test]
    fn switch_case_default() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let default_case = SwitchCase {
            values: &[],
            stmts: &[],
            span: Span::new(1, 0 + 1, 1 - 0),
        };
        assert!(default_case.is_default());

        let expr: &Expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(1),
            span: Span::new(1, 0 + 1, 1 - 0),
        }));
        let values_vec = bumpalo::vec![in &arena; expr];
        let values: &[&Expr] = values_vec.into_bump_slice();
        let case_1 = SwitchCase {
            values,
            stmts: &[],
            span: Span::new(1, 0 + 1, 1 - 0),
        };
        assert!(!case_1.is_default());
    }

    #[test]
    fn for_init_variants() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(0),
            span: Span::new(1, 0 + 1, 1 - 0),
        }));
        let expr_init = ForInit::Expr(expr);
        assert!(matches!(expr_init, ForInit::Expr(_)));
    }

    #[test]
    fn all_stmt_span_variants() {
        use crate::ast::types::{PrimitiveType, TypeExpr};
        use bumpalo::Bump;

        let arena = Bump::new();

        // ExprStmt
        let expr_stmt = Stmt::Expr(ExprStmt {
            expr: None,
            span: Span::new(1, 1, 1),
        });
        assert_eq!(expr_stmt.span(), Span::new(1, 1, 1));

        // VarDecl
        let var_decl = Stmt::VarDecl(VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            vars: &[],
            span: Span::new(1, 1, 10),
        });
        assert_eq!(var_decl.span(), Span::new(1, 1, 10));

        // Return
        let return_stmt = Stmt::Return(ReturnStmt {
            value: None,
            span: Span::new(1, 1, 7),
        });
        assert_eq!(return_stmt.span(), Span::new(1, 1, 7));

        // Break
        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::new(1, 1, 6),
        });
        assert_eq!(break_stmt.span(), Span::new(1, 1, 6));

        // Continue
        let continue_stmt = Stmt::Continue(ContinueStmt {
            span: Span::new(1, 1, 9),
        });
        assert_eq!(continue_stmt.span(), Span::new(1, 1, 9));

        // Block
        let block = Stmt::Block(Block {
            stmts: &[],
            span: Span::new(1, 1, 2),
        });
        assert_eq!(block.span(), Span::new(1, 1, 2));

        // If
        let condition = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Bool(true),
            span: Span::new(1, 4, 4),
        }));
        let then_stmt = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::new(1, 9, 2),
        }));
        let if_stmt = Stmt::If(arena.alloc(IfStmt {
            condition,
            then_stmt,
            else_stmt: None,
            span: Span::new(1, 1, 11),
        }));
        assert_eq!(if_stmt.span(), Span::new(1, 1, 11));

        // While
        let condition = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Bool(true),
            span: Span::new(1, 7, 4),
        }));
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::new(1, 12, 2),
        }));
        let while_stmt = Stmt::While(arena.alloc(WhileStmt {
            condition,
            body,
            span: Span::new(1, 1, 14),
        }));
        assert_eq!(while_stmt.span(), Span::new(1, 1, 14));

        // DoWhile
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::new(1, 4, 2),
        }));
        let condition = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Bool(true),
            span: Span::new(1, 13, 4),
        }));
        let do_while = Stmt::DoWhile(arena.alloc(DoWhileStmt {
            body,
            condition,
            span: Span::new(1, 1, 18),
        }));
        assert_eq!(do_while.span(), Span::new(1, 1, 18));

        // For
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::new(1, 10, 2),
        }));
        let for_stmt = Stmt::For(arena.alloc(ForStmt {
            init: None,
            condition: None,
            update: &[],
            body,
            span: Span::new(1, 1, 12),
        }));
        assert_eq!(for_stmt.span(), Span::new(1, 1, 12));

        // Foreach
        let expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(0),
            span: Span::new(1, 10, 1),
        }));
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::new(1, 12, 2),
        }));
        let foreach = Stmt::Foreach(arena.alloc(ForeachStmt {
            vars: &[],
            expr,
            body,
            span: Span::new(1, 1, 14),
        }));
        assert_eq!(foreach.span(), Span::new(1, 1, 14));

        // Switch
        let expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(0),
            span: Span::new(1, 8, 1),
        }));
        let switch = Stmt::Switch(arena.alloc(SwitchStmt {
            expr,
            cases: &[],
            span: Span::new(1, 1, 15),
        }));
        assert_eq!(switch.span(), Span::new(1, 1, 15));

        // TryCatch
        let try_catch = Stmt::TryCatch(arena.alloc(TryCatchStmt {
            try_block: Block {
                stmts: &[],
                span: Span::new(1, 5, 2),
            },
            catch_block: Block {
                stmts: &[],
                span: Span::new(1, 15, 2),
            },
            span: Span::new(1, 1, 17),
        }));
        assert_eq!(try_catch.span(), Span::new(1, 1, 17));
    }

    #[test]
    fn switch_case_multiple_values() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let expr1: &Expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(1),
            span: Span::new(1, 6, 1),
        }));
        let expr2: &Expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(2),
            span: Span::new(1, 14, 1),
        }));
        let values_vec = bumpalo::vec![in &arena; expr1, expr2];
        let values: &[&Expr] = values_vec.into_bump_slice();
        let case = SwitchCase {
            values,
            stmts: &[],
            span: Span::new(1, 1, 20),
        };
        assert!(!case.is_default());
        assert_eq!(case.values.len(), 2);
    }

    #[test]
    fn expr_stmt_with_expr() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let expr = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(42),
            span: Span::new(1, 1, 2),
        }));
        let stmt = ExprStmt {
            expr: Some(expr),
            span: Span::new(1, 1, 3),
        };
        assert!(stmt.expr.is_some());
    }

    #[test]
    fn var_declarator_with_init() {
        use bumpalo::Bump;
        let arena = Bump::new();

        let init = arena.alloc(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(10),
            span: Span::new(1, 9, 2),
        }));
        let decl = VarDeclarator {
            name: Ident::new("x", Span::new(1, 5, 1)),
            init: Some(init),
            span: Span::new(1, 5, 6),
        };
        assert!(decl.init.is_some());
    }

    #[test]
    fn foreach_var_structure() {
        use crate::ast::types::PrimitiveType;

        let var = ForeachVar {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            name: Ident::new("i", Span::new(1, 5, 1)),
            span: Span::new(1, 1, 6),
        };
        assert_eq!(var.name.name, "i");
    }
}
