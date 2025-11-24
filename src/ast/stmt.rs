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
use crate::lexer::Span;

/// A statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Expression statement (expr;)
    Expr(ExprStmt),
    /// Variable declaration
    VarDecl(VarDeclStmt),
    /// Return statement
    Return(ReturnStmt),
    /// Break statement
    Break(BreakStmt),
    /// Continue statement
    Continue(ContinueStmt),
    /// Block statement
    Block(Block),
    /// If statement
    If(Box<IfStmt>),
    /// While loop
    While(Box<WhileStmt>),
    /// Do-while loop
    DoWhile(Box<DoWhileStmt>),
    /// For loop
    For(Box<ForStmt>),
    /// Foreach loop
    Foreach(Box<ForeachStmt>),
    /// Switch statement
    Switch(Box<SwitchStmt>),
    /// Try-catch statement
    TryCatch(Box<TryCatchStmt>),
}

impl Stmt {
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
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    /// The expression (can be None for empty statement `;`)
    pub expr: Option<Expr>,
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
#[derive(Debug, Clone, PartialEq)]
pub struct VarDeclStmt {
    /// The type of the variable(s)
    pub ty: TypeExpr,
    /// Variable declarations (can be multiple)
    pub vars: Vec<VarDeclarator>,
    /// Source location
    pub span: Span,
}

/// A single variable declarator within a variable declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct VarDeclarator {
    /// Variable name
    pub name: Ident,
    /// Optional initializer
    pub init: Option<Expr>,
    /// Source location
    pub span: Span,
}

/// A return statement.
///
/// Examples:
/// - `return;`
/// - `return expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    /// Optional return value
    pub value: Option<Expr>,
    /// Source location
    pub span: Span,
}

/// A break statement.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakStmt {
    /// Source location
    pub span: Span,
}

/// A continue statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinueStmt {
    /// Source location
    pub span: Span,
}

/// A block of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Statements in the block
    pub stmts: Vec<Stmt>,
    /// Source location
    pub span: Span,
}

/// An if statement.
///
/// Examples:
/// - `if (condition) statement`
/// - `if (condition) statement else statement`
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    /// Condition
    pub condition: Expr,
    /// Then branch
    pub then_stmt: Stmt,
    /// Optional else branch
    pub else_stmt: Option<Stmt>,
    /// Source location
    pub span: Span,
}

/// A while loop.
///
/// Example: `while (condition) statement`
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    /// Condition
    pub condition: Expr,
    /// Body
    pub body: Stmt,
    /// Source location
    pub span: Span,
}

/// A do-while loop.
///
/// Example: `do statement while (condition);`
#[derive(Debug, Clone, PartialEq)]
pub struct DoWhileStmt {
    /// Body
    pub body: Stmt,
    /// Condition
    pub condition: Expr,
    /// Source location
    pub span: Span,
}

/// A for loop.
///
/// Example: `for (init; condition; update) statement`
///
/// The init can be either a variable declaration or an expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    /// Initializer (variable declaration or expression)
    pub init: Option<ForInit>,
    /// Condition
    pub condition: Option<Expr>,
    /// Update expressions
    pub update: Vec<Expr>,
    /// Body
    pub body: Stmt,
    /// Source location
    pub span: Span,
}

/// The initializer in a for loop.
#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    /// Variable declaration
    VarDecl(VarDeclStmt),
    /// Expression
    Expr(Expr),
}

/// A foreach loop.
///
/// Example: `foreach (Type var : expr) statement`
///
/// AngelScript also supports multiple iteration variables:
/// `foreach (Type1 var1, Type2 var2 : expr) statement`
#[derive(Debug, Clone, PartialEq)]
pub struct ForeachStmt {
    /// Iteration variables
    pub vars: Vec<ForeachVar>,
    /// Expression to iterate over
    pub expr: Expr,
    /// Body
    pub body: Stmt,
    /// Source location
    pub span: Span,
}

/// A foreach iteration variable.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeachVar {
    /// Variable type
    pub ty: TypeExpr,
    /// Variable name
    pub name: Ident,
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
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt {
    /// Expression to switch on
    pub expr: Expr,
    /// Cases
    pub cases: Vec<SwitchCase>,
    /// Source location
    pub span: Span,
}

/// A switch case.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    /// Case values (empty for default)
    pub values: Vec<Expr>,
    /// Statements
    pub stmts: Vec<Stmt>,
    /// Source location
    pub span: Span,
}

impl SwitchCase {
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
#[derive(Debug, Clone, PartialEq)]
pub struct TryCatchStmt {
    /// Try block
    pub try_block: Block,
    /// Catch block
    pub catch_block: Block,
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
        let default_case = SwitchCase {
            values: Vec::new(),
            stmts: Vec::new(),
            span: Span::new(1, 0 + 1, 1 - 0),
        };
        assert!(default_case.is_default());

        let case_1 = SwitchCase {
            values: vec![Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Int(1),
                span: Span::new(1, 0 + 1, 1 - 0),
            })],
            stmts: Vec::new(),
            span: Span::new(1, 0 + 1, 1 - 0),
        };
        assert!(!case_1.is_default());
    }

    #[test]
    fn for_init_variants() {
        let expr_init = ForInit::Expr(Expr::Literal(crate::ast::expr::LiteralExpr {
            kind: crate::ast::expr::LiteralKind::Int(0),
            span: Span::new(1, 0 + 1, 1 - 0),
        }));
        assert!(matches!(expr_init, ForInit::Expr(_)));
    }

    #[test]
    fn all_stmt_span_variants() {
        use crate::ast::types::{TypeExpr, PrimitiveType};

        // ExprStmt
        let expr_stmt = Stmt::Expr(ExprStmt {
            expr: None,
            span: Span::new(1, 1, 1),
        });
        assert_eq!(expr_stmt.span(), Span::new(1, 1, 1));

        // VarDecl
        let var_decl = Stmt::VarDecl(VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 1, 3)),
            vars: Vec::new(),
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
            stmts: Vec::new(),
            span: Span::new(1, 1, 2),
        });
        assert_eq!(block.span(), Span::new(1, 1, 2));

        // If
        let if_stmt = Stmt::If(Box::new(IfStmt {
            condition: Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Bool(true),
                span: Span::new(1, 4, 4),
            }),
            then_stmt: Stmt::Block(Block {
                stmts: Vec::new(),
                span: Span::new(1, 9, 2),
            }),
            else_stmt: None,
            span: Span::new(1, 1, 11),
        }));
        assert_eq!(if_stmt.span(), Span::new(1, 1, 11));

        // While
        let while_stmt = Stmt::While(Box::new(WhileStmt {
            condition: Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Bool(true),
                span: Span::new(1, 7, 4),
            }),
            body: Stmt::Block(Block {
                stmts: Vec::new(),
                span: Span::new(1, 12, 2),
            }),
            span: Span::new(1, 1, 14),
        }));
        assert_eq!(while_stmt.span(), Span::new(1, 1, 14));

        // DoWhile
        let do_while = Stmt::DoWhile(Box::new(DoWhileStmt {
            body: Stmt::Block(Block {
                stmts: Vec::new(),
                span: Span::new(1, 4, 2),
            }),
            condition: Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Bool(true),
                span: Span::new(1, 13, 4),
            }),
            span: Span::new(1, 1, 18),
        }));
        assert_eq!(do_while.span(), Span::new(1, 1, 18));

        // For
        let for_stmt = Stmt::For(Box::new(ForStmt {
            init: None,
            condition: None,
            update: Vec::new(),
            body: Stmt::Block(Block {
                stmts: Vec::new(),
                span: Span::new(1, 10, 2),
            }),
            span: Span::new(1, 1, 12),
        }));
        assert_eq!(for_stmt.span(), Span::new(1, 1, 12));

        // Foreach
        let foreach = Stmt::Foreach(Box::new(ForeachStmt {
            vars: Vec::new(),
            expr: Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Int(0),
                span: Span::new(1, 10, 1),
            }),
            body: Stmt::Block(Block {
                stmts: Vec::new(),
                span: Span::new(1, 12, 2),
            }),
            span: Span::new(1, 1, 14),
        }));
        assert_eq!(foreach.span(), Span::new(1, 1, 14));

        // Switch
        let switch = Stmt::Switch(Box::new(SwitchStmt {
            expr: Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Int(0),
                span: Span::new(1, 8, 1),
            }),
            cases: Vec::new(),
            span: Span::new(1, 1, 15),
        }));
        assert_eq!(switch.span(), Span::new(1, 1, 15));

        // TryCatch
        let try_catch = Stmt::TryCatch(Box::new(TryCatchStmt {
            try_block: Block {
                stmts: Vec::new(),
                span: Span::new(1, 5, 2),
            },
            catch_block: Block {
                stmts: Vec::new(),
                span: Span::new(1, 15, 2),
            },
            span: Span::new(1, 1, 17),
        }));
        assert_eq!(try_catch.span(), Span::new(1, 1, 17));
    }

    #[test]
    fn switch_case_multiple_values() {
        let case = SwitchCase {
            values: vec![
                Expr::Literal(crate::ast::expr::LiteralExpr {
                    kind: crate::ast::expr::LiteralKind::Int(1),
                    span: Span::new(1, 6, 1),
                }),
                Expr::Literal(crate::ast::expr::LiteralExpr {
                    kind: crate::ast::expr::LiteralKind::Int(2),
                    span: Span::new(1, 14, 1),
                }),
            ],
            stmts: Vec::new(),
            span: Span::new(1, 1, 20),
        };
        assert!(!case.is_default());
        assert_eq!(case.values.len(), 2);
    }

    #[test]
    fn expr_stmt_with_expr() {
        let stmt = ExprStmt {
            expr: Some(Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Int(42),
                span: Span::new(1, 1, 2),
            })),
            span: Span::new(1, 1, 3),
        };
        assert!(stmt.expr.is_some());
    }

    #[test]
    fn var_declarator_with_init() {
        let decl = VarDeclarator {
            name: Ident::new("x", Span::new(1, 5, 1)),
            init: Some(Expr::Literal(crate::ast::expr::LiteralExpr {
                kind: crate::ast::expr::LiteralKind::Int(10),
                span: Span::new(1, 9, 2),
            })),
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
