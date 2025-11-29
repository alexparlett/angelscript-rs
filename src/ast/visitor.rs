//! Visitor pattern for traversing the AST.
//!
//! This module provides a `Visitor` trait and corresponding `walk_*` functions
//! that enable easy traversal and analysis of AngelScript AST nodes.
//!
//! # Example: Using the Visitor Pattern
//!
//! ```
//! use angelscript::{parse_lenient, visitor::Visitor, FunctionDecl, Item, Script};
//! use bumpalo::Bump;
//!
//! struct FunctionCounter {
//!     count: usize,
//! }
//!
//! impl<'src, 'ast> Visitor<'src, 'ast> for FunctionCounter {
//!     // Override the function visit method
//! }
//!
//! let arena = Bump::new();
//! let source = "void foo() {} void bar() {}";
//! let (script, _) = parse_lenient(source, &arena);
//!
//! // Or just count directly
//! let func_count = script.items().iter()
//!     .filter(|item| matches!(item, Item::Function(_)))
//!     .count();
//!
//! println!("Found {} functions", func_count);
//! ```
//!
//! The visitor pattern allows for extensible AST traversal without
//! modifying the AST types themselves.

use crate::ast::decl::*;
use crate::ast::expr::*;
use crate::ast::stmt::*;
use crate::ast::types::*;
use crate::ast::Script;

/// Visitor trait for traversing AST nodes.
///
/// Each `visit_*` method corresponds to an AST node type and is called
/// when that node is encountered during traversal. The default implementations
/// call the corresponding `walk_*` function to continue traversal.
///
/// Override any `visit_*` method to customize behavior for specific node types.
///
/// The visitor is parameterized by lifetimes `'src` and `'ast` to allow visitors
/// to store references to AST data.
pub trait Visitor<'src, 'ast>: Sized {
    // === Script and Items ===

    /// Visit a script (root node).
    fn visit_script(&mut self, script: &Script<'src, 'ast>) {
        walk_script(self, script);
    }

    /// Visit a top-level item.
    fn visit_item(&mut self, item: &Item<'src, 'ast>) {
        walk_item(self, item);
    }

    /// Visit a function declaration.
    fn visit_function_decl(&mut self, func: &FunctionDecl<'src, 'ast>) {
        walk_function_decl(self, func);
    }

    /// Visit a class declaration.
    fn visit_class_decl(&mut self, class: &ClassDecl<'src, 'ast>) {
        walk_class_decl(self, class);
    }

    /// Visit an interface declaration.
    fn visit_interface_decl(&mut self, interface: &InterfaceDecl<'src, 'ast>) {
        walk_interface_decl(self, interface);
    }

    /// Visit an enum declaration.
    fn visit_enum_decl(&mut self, enum_decl: &EnumDecl<'src, 'ast>) {
        walk_enum_decl(self, enum_decl);
    }

    /// Visit a global variable declaration.
    fn visit_global_var_decl(&mut self, var: &GlobalVarDecl<'src, 'ast>) {
        walk_global_var_decl(self, var);
    }

    /// Visit a namespace declaration.
    fn visit_namespace_decl(&mut self, namespace: &NamespaceDecl<'src, 'ast>) {
        walk_namespace_decl(self, namespace);
    }

    /// Visit a typedef declaration.
    fn visit_typedef_decl(&mut self, typedef: &TypedefDecl<'src, 'ast>) {
        walk_typedef_decl(self, typedef);
    }

    /// Visit a funcdef declaration.
    fn visit_funcdef_decl(&mut self, funcdef: &FuncdefDecl<'src, 'ast>) {
        walk_funcdef_decl(self, funcdef);
    }

    /// Visit a mixin declaration.
    fn visit_mixin_decl(&mut self, mixin: &MixinDecl<'src, 'ast>) {
        walk_mixin_decl(self, mixin);
    }

    /// Visit an import declaration.
    fn visit_import_decl(&mut self, import: &ImportDecl<'src, 'ast>) {
        walk_import_decl(self, import);
    }

    // === Class Members ===

    /// Visit a class member.
    fn visit_class_member(&mut self, member: &ClassMember<'src, 'ast>) {
        walk_class_member(self, member);
    }

    /// Visit a field declaration.
    fn visit_field_decl(&mut self, field: &FieldDecl<'src, 'ast>) {
        walk_field_decl(self, field);
    }

    /// Visit a virtual property declaration.
    fn visit_virtual_property_decl(&mut self, prop: &VirtualPropertyDecl<'src, 'ast>) {
        walk_virtual_property_decl(self, prop);
    }

    /// Visit a property accessor.
    fn visit_property_accessor(&mut self, accessor: &PropertyAccessor<'src, 'ast>) {
        walk_property_accessor(self, accessor);
    }

    // === Interface Members ===

    /// Visit an interface member.
    fn visit_interface_member(&mut self, member: &InterfaceMember<'src, 'ast>) {
        walk_interface_member(self, member);
    }

    /// Visit an interface method.
    fn visit_interface_method(&mut self, method: &InterfaceMethod<'src, 'ast>) {
        walk_interface_method(self, method);
    }

    // === Statements ===

    /// Visit a statement.
    fn visit_stmt(&mut self, stmt: &Stmt<'src, 'ast>) {
        walk_stmt(self, stmt);
    }

    /// Visit an expression statement.
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt<'src, 'ast>) {
        walk_expr_stmt(self, stmt);
    }

    /// Visit a variable declaration statement.
    fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt<'src, 'ast>) {
        walk_var_decl_stmt(self, stmt);
    }

    /// Visit a return statement.
    fn visit_return_stmt(&mut self, stmt: &ReturnStmt<'src, 'ast>) {
        walk_return_stmt(self, stmt);
    }

    /// Visit a break statement.
    fn visit_break_stmt(&mut self, _stmt: &BreakStmt) {
        // Leaf node, no children
    }

    /// Visit a continue statement.
    fn visit_continue_stmt(&mut self, _stmt: &ContinueStmt) {
        // Leaf node, no children
    }

    /// Visit a block.
    fn visit_block(&mut self, block: &Block<'src, 'ast>) {
        walk_block(self, block);
    }

    /// Visit an if statement.
    fn visit_if_stmt(&mut self, stmt: &IfStmt<'src, 'ast>) {
        walk_if_stmt(self, stmt);
    }

    /// Visit a while statement.
    fn visit_while_stmt(&mut self, stmt: &WhileStmt<'src, 'ast>) {
        walk_while_stmt(self, stmt);
    }

    /// Visit a do-while statement.
    fn visit_do_while_stmt(&mut self, stmt: &DoWhileStmt<'src, 'ast>) {
        walk_do_while_stmt(self, stmt);
    }

    /// Visit a for statement.
    fn visit_for_stmt(&mut self, stmt: &ForStmt<'src, 'ast>) {
        walk_for_stmt(self, stmt);
    }

    /// Visit a foreach statement.
    fn visit_foreach_stmt(&mut self, stmt: &ForeachStmt<'src, 'ast>) {
        walk_foreach_stmt(self, stmt);
    }

    /// Visit a switch statement.
    fn visit_switch_stmt(&mut self, stmt: &SwitchStmt<'src, 'ast>) {
        walk_switch_stmt(self, stmt);
    }

    /// Visit a try-catch statement.
    fn visit_try_catch_stmt(&mut self, stmt: &TryCatchStmt<'src, 'ast>) {
        walk_try_catch_stmt(self, stmt);
    }

    // === Expressions ===

    /// Visit an expression.
    fn visit_expr(&mut self, expr: &Expr<'src, 'ast>) {
        walk_expr(self, expr);
    }

    /// Visit a literal expression.
    fn visit_literal_expr(&mut self, _expr: &LiteralExpr) {
        // Leaf node, no children
    }

    /// Visit an identifier expression.
    fn visit_ident_expr(&mut self, _expr: &IdentExpr<'src, 'ast>) {
        // Leaf node, no children
    }

    /// Visit a binary expression.
    fn visit_binary_expr(&mut self, expr: &BinaryExpr<'src, 'ast>) {
        walk_binary_expr(self, expr);
    }

    /// Visit a unary expression.
    fn visit_unary_expr(&mut self, expr: &UnaryExpr<'src, 'ast>) {
        walk_unary_expr(self, expr);
    }

    /// Visit an assignment expression.
    fn visit_assign_expr(&mut self, expr: &AssignExpr<'src, 'ast>) {
        walk_assign_expr(self, expr);
    }

    /// Visit a ternary expression.
    fn visit_ternary_expr(&mut self, expr: &TernaryExpr<'src, 'ast>) {
        walk_ternary_expr(self, expr);
    }

    /// Visit a call expression.
    fn visit_call_expr(&mut self, expr: &CallExpr<'src, 'ast>) {
        walk_call_expr(self, expr);
    }

    /// Visit an index expression.
    fn visit_index_expr(&mut self, expr: &IndexExpr<'src, 'ast>) {
        walk_index_expr(self, expr);
    }

    /// Visit a member expression.
    fn visit_member_expr(&mut self, expr: &MemberExpr<'src, 'ast>) {
        walk_member_expr(self, expr);
    }

    /// Visit a postfix expression.
    fn visit_postfix_expr(&mut self, expr: &PostfixExpr<'src, 'ast>) {
        walk_postfix_expr(self, expr);
    }

    /// Visit a cast expression.
    fn visit_cast_expr(&mut self, expr: &CastExpr<'src, 'ast>) {
        walk_cast_expr(self, expr);
    }

    /// Visit a lambda expression.
    fn visit_lambda_expr(&mut self, expr: &LambdaExpr<'src, 'ast>) {
        walk_lambda_expr(self, expr);
    }

    /// Visit an initializer list expression.
    fn visit_init_list_expr(&mut self, expr: &InitListExpr<'src, 'ast>) {
        walk_init_list_expr(self, expr);
    }

    /// Visit a parenthesized expression.
    fn visit_paren_expr(&mut self, expr: &ParenExpr<'src, 'ast>) {
        walk_paren_expr(self, expr);
    }

    // === Types ===

    /// Visit a type expression.
    fn visit_type_expr(&mut self, ty: &TypeExpr<'src, 'ast>) {
        walk_type_expr(self, ty);
    }

    /// Visit a parameter type.
    fn visit_param_type(&mut self, ty: &ParamType<'src, 'ast>) {
        walk_param_type(self, ty);
    }

    /// Visit a return type.
    fn visit_return_type(&mut self, ty: &ReturnType<'src, 'ast>) {
        walk_return_type(self, ty);
    }
}

// === Walk Functions ===
// These provide default traversal logic for each node type.

/// Walk a script (root node).
pub fn walk_script<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, script: &Script<'src, 'ast>) {
    for item in script.items() {
        visitor.visit_item(item);
    }
}

/// Walk a top-level item.
pub fn walk_item<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, item: &Item<'src, 'ast>) {
    match item {
        Item::Function(func) => visitor.visit_function_decl(func),
        Item::Class(class) => visitor.visit_class_decl(class),
        Item::Interface(interface) => visitor.visit_interface_decl(interface),
        Item::Enum(enum_decl) => visitor.visit_enum_decl(enum_decl),
        Item::GlobalVar(var) => visitor.visit_global_var_decl(var),
        Item::Namespace(namespace) => visitor.visit_namespace_decl(namespace),
        Item::Typedef(typedef) => visitor.visit_typedef_decl(typedef),
        Item::Funcdef(funcdef) => visitor.visit_funcdef_decl(funcdef),
        Item::Mixin(mixin) => visitor.visit_mixin_decl(mixin),
        Item::Import(import) => visitor.visit_import_decl(import),
    }
}

/// Walk a function declaration.
pub fn walk_function_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, func: &FunctionDecl<'src, 'ast>) {
    // Visit return type
    if let Some(return_type) = &func.return_type {
        visitor.visit_return_type(return_type);
    }

    // Visit parameters
    for param in func.params {
        visitor.visit_param_type(&param.ty);
        if let Some(default) = &param.default {
            visitor.visit_expr(default);
        }
    }

    // Visit body
    if let Some(body) = &func.body {
        visitor.visit_block(body);
    }
}

/// Walk a class declaration.
pub fn walk_class_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, class: &ClassDecl<'src, 'ast>) {
    for member in class.members {
        visitor.visit_class_member(member);
    }
}

/// Walk an interface declaration.
pub fn walk_interface_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, interface: &InterfaceDecl<'src, 'ast>) {
    for member in interface.members {
        visitor.visit_interface_member(member);
    }
}

/// Walk an enum declaration.
pub fn walk_enum_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, enum_decl: &EnumDecl<'src, 'ast>) {
    for enumerator in enum_decl.enumerators {
        if let Some(value) = &enumerator.value {
            visitor.visit_expr(value);
        }
    }
}

/// Walk a global variable declaration.
pub fn walk_global_var_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, var: &GlobalVarDecl<'src, 'ast>) {
    visitor.visit_type_expr(&var.ty);
    if let Some(init) = &var.init {
        visitor.visit_expr(init);
    }
}

/// Walk a namespace declaration.
pub fn walk_namespace_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, namespace: &NamespaceDecl<'src, 'ast>) {
    for item in namespace.items {
        visitor.visit_item(item);
    }
}

/// Walk a typedef declaration.
pub fn walk_typedef_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, typedef: &TypedefDecl<'src, 'ast>) {
    visitor.visit_type_expr(&typedef.base_type);
}

/// Walk a funcdef declaration.
pub fn walk_funcdef_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, funcdef: &FuncdefDecl<'src, 'ast>) {
    visitor.visit_return_type(&funcdef.return_type);
    for param in funcdef.params {
        visitor.visit_param_type(&param.ty);
        if let Some(default) = &param.default {
            visitor.visit_expr(default);
        }
    }
}

/// Walk a mixin declaration.
pub fn walk_mixin_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, mixin: &MixinDecl<'src, 'ast>) {
    visitor.visit_class_decl(&mixin.class);
}

/// Walk an import declaration.
pub fn walk_import_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, import: &ImportDecl<'src, 'ast>) {
    visitor.visit_return_type(&import.return_type);
    for param in import.params {
        visitor.visit_param_type(&param.ty);
        if let Some(default) = &param.default {
            visitor.visit_expr(default);
        }
    }
}

// === Class Members ===

/// Walk a class member.
pub fn walk_class_member<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, member: &ClassMember<'src, 'ast>) {
    match member {
        ClassMember::Method(method) => visitor.visit_function_decl(method),
        ClassMember::Field(field) => visitor.visit_field_decl(field),
        ClassMember::VirtualProperty(prop) => visitor.visit_virtual_property_decl(prop),
        ClassMember::Funcdef(funcdef) => visitor.visit_funcdef_decl(funcdef),
    }
}

/// Walk a field declaration.
pub fn walk_field_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, field: &FieldDecl<'src, 'ast>) {
    visitor.visit_type_expr(&field.ty);
    if let Some(init) = &field.init {
        visitor.visit_expr(init);
    }
}

/// Walk a virtual property declaration.
pub fn walk_virtual_property_decl<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, prop: &VirtualPropertyDecl<'src, 'ast>) {
    visitor.visit_return_type(&prop.ty);
    for accessor in prop.accessors {
        visitor.visit_property_accessor(accessor);
    }
}

/// Walk a property accessor.
pub fn walk_property_accessor<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, accessor: &PropertyAccessor<'src, 'ast>) {
    if let Some(body) = &accessor.body {
        visitor.visit_block(body);
    }
}

// === Interface Members ===

/// Walk an interface member.
pub fn walk_interface_member<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, member: &InterfaceMember<'src, 'ast>) {
    match member {
        InterfaceMember::Method(method) => visitor.visit_interface_method(method),
        InterfaceMember::VirtualProperty(prop) => visitor.visit_virtual_property_decl(prop),
    }
}

/// Walk an interface method.
pub fn walk_interface_method<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, method: &InterfaceMethod<'src, 'ast>) {
    visitor.visit_return_type(&method.return_type);
    for param in method.params {
        visitor.visit_param_type(&param.ty);
        if let Some(default) = &param.default {
            visitor.visit_expr(default);
        }
    }
}

// === Statements ===

/// Walk a statement.
pub fn walk_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &Stmt<'src, 'ast>) {
    match stmt {
        Stmt::Expr(s) => visitor.visit_expr_stmt(s),
        Stmt::VarDecl(s) => visitor.visit_var_decl_stmt(s),
        Stmt::Return(s) => visitor.visit_return_stmt(s),
        Stmt::Break(s) => visitor.visit_break_stmt(s),
        Stmt::Continue(s) => visitor.visit_continue_stmt(s),
        Stmt::Block(s) => visitor.visit_block(s),
        Stmt::If(s) => visitor.visit_if_stmt(s),
        Stmt::While(s) => visitor.visit_while_stmt(s),
        Stmt::DoWhile(s) => visitor.visit_do_while_stmt(s),
        Stmt::For(s) => visitor.visit_for_stmt(s),
        Stmt::Foreach(s) => visitor.visit_foreach_stmt(s),
        Stmt::Switch(s) => visitor.visit_switch_stmt(s),
        Stmt::TryCatch(s) => visitor.visit_try_catch_stmt(s),
    }
}

/// Walk an expression statement.
pub fn walk_expr_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &ExprStmt<'src, 'ast>) {
    if let Some(expr) = &stmt.expr {
        visitor.visit_expr(expr);
    }
}

/// Walk a variable declaration statement.
pub fn walk_var_decl_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &VarDeclStmt<'src, 'ast>) {
    visitor.visit_type_expr(&stmt.ty);
    for var in stmt.vars {
        if let Some(init) = &var.init {
            visitor.visit_expr(init);
        }
    }
}

/// Walk a return statement.
pub fn walk_return_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &ReturnStmt<'src, 'ast>) {
    if let Some(value) = &stmt.value {
        visitor.visit_expr(value);
    }
}

/// Walk a block.
pub fn walk_block<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, block: &Block<'src, 'ast>) {
    for stmt in block.stmts {
        visitor.visit_stmt(stmt);
    }
}

/// Walk an if statement.
pub fn walk_if_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &IfStmt<'src, 'ast>) {
    visitor.visit_expr(stmt.condition);
    visitor.visit_stmt(stmt.then_stmt);
    if let Some(else_stmt) = &stmt.else_stmt {
        visitor.visit_stmt(else_stmt);
    }
}

/// Walk a while statement.
pub fn walk_while_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &WhileStmt<'src, 'ast>) {
    visitor.visit_expr(stmt.condition);
    visitor.visit_stmt(stmt.body);
}

/// Walk a do-while statement.
pub fn walk_do_while_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &DoWhileStmt<'src, 'ast>) {
    visitor.visit_stmt(stmt.body);
    visitor.visit_expr(stmt.condition);
}

/// Walk a for statement.
pub fn walk_for_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &ForStmt<'src, 'ast>) {
    // Visit init
    if let Some(init) = &stmt.init {
        match init {
            ForInit::VarDecl(var_decl) => visitor.visit_var_decl_stmt(var_decl),
            ForInit::Expr(expr) => visitor.visit_expr(expr),
        }
    }

    // Visit condition
    if let Some(condition) = &stmt.condition {
        visitor.visit_expr(condition);
    }

    // Visit update
    for expr in stmt.update {
        visitor.visit_expr(expr);
    }

    // Visit body
    visitor.visit_stmt(stmt.body);
}

/// Walk a foreach statement.
pub fn walk_foreach_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &ForeachStmt<'src, 'ast>) {
    // Visit variable types
    for var in stmt.vars {
        visitor.visit_type_expr(&var.ty);
    }

    // Visit iterable expression
    visitor.visit_expr(stmt.expr);

    // Visit body
    visitor.visit_stmt(stmt.body);
}

/// Walk a switch statement.
pub fn walk_switch_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &SwitchStmt<'src, 'ast>) {
    visitor.visit_expr(stmt.expr);
    for case in stmt.cases {
        for value in case.values {
            visitor.visit_expr(value);
        }
        for stmt in case.stmts {
            visitor.visit_stmt(stmt);
        }
    }
}

/// Walk a try-catch statement.
pub fn walk_try_catch_stmt<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, stmt: &TryCatchStmt<'src, 'ast>) {
    visitor.visit_block(&stmt.try_block);
    visitor.visit_block(&stmt.catch_block);
}

// === Expressions ===

/// Walk an expression.
pub fn walk_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &Expr<'src, 'ast>) {
    match expr {
        Expr::Literal(e) => visitor.visit_literal_expr(e),
        Expr::Ident(e) => visitor.visit_ident_expr(e),
        Expr::Binary(e) => visitor.visit_binary_expr(e),
        Expr::Unary(e) => visitor.visit_unary_expr(e),
        Expr::Assign(e) => visitor.visit_assign_expr(e),
        Expr::Ternary(e) => visitor.visit_ternary_expr(e),
        Expr::Call(e) => visitor.visit_call_expr(e),
        Expr::Index(e) => visitor.visit_index_expr(e),
        Expr::Member(e) => visitor.visit_member_expr(e),
        Expr::Postfix(e) => visitor.visit_postfix_expr(e),
        Expr::Cast(e) => visitor.visit_cast_expr(e),
        Expr::Lambda(e) => visitor.visit_lambda_expr(e),
        Expr::InitList(e) => visitor.visit_init_list_expr(e),
        Expr::Paren(e) => visitor.visit_paren_expr(e),
    }
}

/// Walk a binary expression.
pub fn walk_binary_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &BinaryExpr<'src, 'ast>) {
    visitor.visit_expr(expr.left);
    visitor.visit_expr(expr.right);
}

/// Walk a unary expression.
pub fn walk_unary_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &UnaryExpr<'src, 'ast>) {
    visitor.visit_expr(expr.operand);
}

/// Walk an assignment expression.
pub fn walk_assign_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &AssignExpr<'src, 'ast>) {
    visitor.visit_expr(expr.target);
    visitor.visit_expr(expr.value);
}

/// Walk a ternary expression.
pub fn walk_ternary_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &TernaryExpr<'src, 'ast>) {
    visitor.visit_expr(expr.condition);
    visitor.visit_expr(expr.then_expr);
    visitor.visit_expr(expr.else_expr);
}

/// Walk a call expression.
pub fn walk_call_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &CallExpr<'src, 'ast>) {
    visitor.visit_expr(expr.callee);
    for arg in expr.args {
        visitor.visit_expr(arg.value);
    }
}

/// Walk an index expression.
pub fn walk_index_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &IndexExpr<'src, 'ast>) {
    visitor.visit_expr(expr.object);
    for index in expr.indices {
        visitor.visit_expr(index.index);
    }
}

/// Walk a member expression.
pub fn walk_member_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &MemberExpr<'src, 'ast>) {
    visitor.visit_expr(expr.object);
    if let MemberAccess::Method { args, .. } = &expr.member {
        for arg in *args {
            visitor.visit_expr(arg.value);
        }
    }
}

/// Walk a postfix expression.
pub fn walk_postfix_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &PostfixExpr<'src, 'ast>) {
    visitor.visit_expr(expr.operand);
}

/// Walk a cast expression.
pub fn walk_cast_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &CastExpr<'src, 'ast>) {
    visitor.visit_type_expr(&expr.target_type);
    visitor.visit_expr(expr.expr);
}

/// Walk a lambda expression.
pub fn walk_lambda_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &LambdaExpr<'src, 'ast>) {
    // Visit parameter types
    for param in expr.params {
        if let Some(ty) = &param.ty {
            visitor.visit_param_type(ty);
        }
    }

    // Visit return type
    if let Some(return_type) = &expr.return_type {
        visitor.visit_return_type(return_type);
    }

    // Visit body
    visitor.visit_block(expr.body);
}

/// Walk an initializer list expression.
pub fn walk_init_list_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &InitListExpr<'src, 'ast>) {
    if let Some(ty) = &expr.ty {
        visitor.visit_type_expr(ty);
    }
    for element in expr.elements {
        match element {
            InitElement::Expr(e) => visitor.visit_expr(e),
            InitElement::InitList(list) => visitor.visit_init_list_expr(list),
        }
    }
}

/// Walk a parenthesized expression.
pub fn walk_paren_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, expr: &ParenExpr<'src, 'ast>) {
    visitor.visit_expr(expr.expr);
}

// === Types ===

/// Walk a type expression.
pub fn walk_type_expr<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, ty: &TypeExpr<'src, 'ast>) {
    // Visit template arguments
    for arg in ty.template_args {
        visitor.visit_type_expr(arg);
    }
}

/// Walk a parameter type.
pub fn walk_param_type<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, param: &ParamType<'src, 'ast>) {
    visitor.visit_type_expr(&param.ty);
}

/// Walk a return type.
pub fn walk_return_type<'src, 'ast, V: Visitor<'src, 'ast>>(visitor: &mut V, ty: &ReturnType<'src, 'ast>) {
    visitor.visit_type_expr(&ty.ty);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Ident;
    use crate::lexer::Span;

    /// Example visitor that counts function declarations.
    struct FunctionCounter {
        count: usize,
    }

    impl<'src, 'ast> Visitor<'src, 'ast> for FunctionCounter {
        fn visit_function_decl(&mut self, func: &FunctionDecl<'src, 'ast>) {
            self.count += 1;
            // Continue walking to count nested functions
            walk_function_decl(self, func);
        }
    }

    /// Example visitor that collects all identifier names.
    struct IdentCollector {
        names: Vec<String>,
    }

    impl<'src, 'ast> Visitor<'src, 'ast> for IdentCollector {
        fn visit_ident_expr(&mut self, expr: &IdentExpr<'src, 'ast>) {
            self.names.push(expr.ident.name.to_string());
        }
    }

    /// Generic counter visitor that tracks visits to different node types
    struct NodeCounter {
        functions: usize,
        classes: usize,
        interfaces: usize,
        enums: usize,
        namespaces: usize,
        stmts: usize,
        exprs: usize,
        literals: usize,
        if_stmts: usize,
        while_stmts: usize,
        for_stmts: usize,
        switch_stmts: usize,
        blocks: usize,
        fields: usize,
        methods: usize,
    }

    impl NodeCounter {
        fn new() -> Self {
            Self {
                functions: 0,
                classes: 0,
                interfaces: 0,
                enums: 0,
                namespaces: 0,
                stmts: 0,
                exprs: 0,
                literals: 0,
                if_stmts: 0,
                while_stmts: 0,
                for_stmts: 0,
                switch_stmts: 0,
                blocks: 0,
                fields: 0,
                methods: 0,
            }
        }
    }

    impl<'src, 'ast> Visitor<'src, 'ast> for NodeCounter {
        fn visit_function_decl(&mut self, func: &FunctionDecl<'src, 'ast>) {
            self.functions += 1;
            walk_function_decl(self, func);
        }

        fn visit_class_decl(&mut self, class: &ClassDecl<'src, 'ast>) {
            self.classes += 1;
            walk_class_decl(self, class);
        }

        fn visit_interface_decl(&mut self, interface: &InterfaceDecl<'src, 'ast>) {
            self.interfaces += 1;
            walk_interface_decl(self, interface);
        }

        fn visit_enum_decl(&mut self, enum_decl: &EnumDecl<'src, 'ast>) {
            self.enums += 1;
            walk_enum_decl(self, enum_decl);
        }

        fn visit_namespace_decl(&mut self, namespace: &NamespaceDecl<'src, 'ast>) {
            self.namespaces += 1;
            walk_namespace_decl(self, namespace);
        }

        fn visit_stmt(&mut self, stmt: &Stmt<'src, 'ast>) {
            self.stmts += 1;
            walk_stmt(self, stmt);
        }

        fn visit_expr(&mut self, expr: &Expr<'src, 'ast>) {
            self.exprs += 1;
            walk_expr(self, expr);
        }

        fn visit_literal_expr(&mut self, _expr: &LiteralExpr) {
            self.literals += 1;
        }

        fn visit_if_stmt(&mut self, stmt: &IfStmt<'src, 'ast>) {
            self.if_stmts += 1;
            walk_if_stmt(self, stmt);
        }

        fn visit_while_stmt(&mut self, stmt: &WhileStmt<'src, 'ast>) {
            self.while_stmts += 1;
            walk_while_stmt(self, stmt);
        }

        fn visit_for_stmt(&mut self, stmt: &ForStmt<'src, 'ast>) {
            self.for_stmts += 1;
            walk_for_stmt(self, stmt);
        }

        fn visit_switch_stmt(&mut self, stmt: &SwitchStmt<'src, 'ast>) {
            self.switch_stmts += 1;
            walk_switch_stmt(self, stmt);
        }

        fn visit_block(&mut self, block: &Block<'src, 'ast>) {
            self.blocks += 1;
            walk_block(self, block);
        }

        fn visit_field_decl(&mut self, field: &FieldDecl<'src, 'ast>) {
            self.fields += 1;
            walk_field_decl(self, field);
        }

        fn visit_class_member(&mut self, member: &ClassMember<'src, 'ast>) {
            if matches!(member, ClassMember::Method(_)) {
                self.methods += 1;
            }
            walk_class_member(self, member);
        }
    }

    #[test]
    fn test_function_counter() {
        // Parse a simple script with two functions
        let source = "void foo() {} int bar() { return 0; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse test script");

        let mut counter = FunctionCounter { count: 0 };
        walk_script(&mut counter, &script);
        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_ident_collector() {
        use crate::ast::expr::{Expr, IdentExpr};

        let expr = Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("myVar", Span::new(1, 0 + 1, 5 - 0)),
            span: Span::new(1, 0 + 1, 5 - 0),
        });

        let mut collector = IdentCollector { names: Vec::new() };
        walk_expr(&mut collector, &expr);
        assert_eq!(collector.names, vec!["myVar"]);
    }

    #[test]
    fn test_binary_expr_traversal() {
        use crate::ast::expr::{Expr, BinaryExpr, IdentExpr, LiteralExpr, LiteralKind};
        use crate::ast::BinaryOp;
        use bumpalo::Bump;
        let arena = Bump::new();

        // Create: x + 42
        let expr = Expr::Binary(arena.alloc(BinaryExpr {
            left: arena.alloc(Expr::Ident(IdentExpr {
                scope: None,
                ident: Ident::new("x", Span::new(1, 0 + 1, 1 - 0)),
                span: Span::new(1, 0 + 1, 1 - 0),
            })),
            op: BinaryOp::Add,
            right: arena.alloc(Expr::Literal(LiteralExpr {
                kind: LiteralKind::Int(42),
                span: Span::new(1, 4 + 1, 6 - 4),
            })),
            span: Span::new(1, 0 + 1, 6 - 0),
        }));

        let mut collector = IdentCollector { names: Vec::new() };
        walk_expr(&mut collector, &expr);
        assert_eq!(collector.names, vec!["x"]);
    }

    #[test]
    fn test_visitor_override() {
        // Test that visitor methods can be overridden
        struct CustomVisitor {
            visited_return: bool,
        }

        impl<'src, 'ast> Visitor<'src, 'ast> for CustomVisitor {
            fn visit_return_stmt(&mut self, _stmt: &ReturnStmt<'src, 'ast>) {
                self.visited_return = true;
                // Don't call walk_return_stmt - custom behavior
            }
        }

        let stmt = Stmt::Return(ReturnStmt {
            value: None,
            span: Span::new(1, 0 + 1, 7 - 0),
        });

        let mut visitor = CustomVisitor {
            visited_return: false,
        };
        walk_stmt(&mut visitor, &stmt);
        assert!(visitor.visited_return);
    }

    // ==================== Class and Interface Tests ====================

    #[test]
    fn test_class_traversal() {
        let source = r#"
            class Player {
                int health;
                float speed;
                void move() {}
                int getHealth() { return health; }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.classes, 1);
        assert_eq!(counter.fields, 2);
        assert_eq!(counter.methods, 2);
    }

    #[test]
    fn test_interface_traversal() {
        let source = r#"
            interface IDrawable {
                void draw();
                int getWidth();
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.interfaces, 1);
    }

    #[test]
    fn test_enum_traversal() {
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue = 10
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.enums, 1);
        // Blue = 10 has an expression
        assert!(counter.exprs >= 1);
    }

    #[test]
    fn test_namespace_traversal() {
        let source = r#"
            namespace Game {
                void init() {}
                namespace Utils {
                    int helper() { return 0; }
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.namespaces, 2); // Game and Game::Utils
        assert_eq!(counter.functions, 2);
    }

    // ==================== Statement Tests ====================

    #[test]
    fn test_if_stmt_traversal() {
        let source = r#"
            void test() {
                if (true) {
                    int x = 1;
                } else {
                    int y = 2;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.if_stmts, 1);
        assert!(counter.blocks >= 2); // if block and else block
    }

    #[test]
    fn test_while_stmt_traversal() {
        let source = r#"
            void test() {
                while (true) {
                    break;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.while_stmts, 1);
    }

    #[test]
    fn test_for_stmt_traversal() {
        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    int x = i;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.for_stmts, 1);
    }

    #[test]
    fn test_switch_stmt_traversal() {
        let source = r#"
            void test() {
                int x = 1;
                switch (x) {
                    case 1:
                        break;
                    case 2:
                        break;
                    default:
                        break;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.switch_stmts, 1);
    }

    #[test]
    fn test_do_while_traversal() {
        struct DoWhileCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for DoWhileCounter {
            fn visit_do_while_stmt(&mut self, stmt: &DoWhileStmt<'src, 'ast>) {
                self.count += 1;
                walk_do_while_stmt(self, stmt);
            }
        }

        let source = r#"
            void test() {
                do {
                    int x = 1;
                } while (true);
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = DoWhileCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_try_catch_traversal() {
        struct TryCatchCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for TryCatchCounter {
            fn visit_try_catch_stmt(&mut self, stmt: &TryCatchStmt<'src, 'ast>) {
                self.count += 1;
                walk_try_catch_stmt(self, stmt);
            }
        }

        let source = r#"
            void test() {
                try {
                    int x = 1;
                } catch {
                    int y = 2;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = TryCatchCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    // ==================== Expression Tests ====================

    #[test]
    fn test_unary_expr_traversal() {
        struct UnaryCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for UnaryCounter {
            fn visit_unary_expr(&mut self, expr: &UnaryExpr<'src, 'ast>) {
                self.count += 1;
                walk_unary_expr(self, expr);
            }
        }

        let source = "void test() { int x = -5; bool b = !true; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = UnaryCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_ternary_expr_traversal() {
        struct TernaryCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for TernaryCounter {
            fn visit_ternary_expr(&mut self, expr: &TernaryExpr<'src, 'ast>) {
                self.count += 1;
                walk_ternary_expr(self, expr);
            }
        }

        let source = "void test() { int x = true ? 1 : 2; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = TernaryCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_call_expr_traversal() {
        struct CallCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for CallCounter {
            fn visit_call_expr(&mut self, expr: &CallExpr<'src, 'ast>) {
                self.count += 1;
                walk_call_expr(self, expr);
            }
        }

        let source = "void test() { foo(); bar(1, 2); }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = CallCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_index_expr_traversal() {
        struct IndexCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for IndexCounter {
            fn visit_index_expr(&mut self, expr: &IndexExpr<'src, 'ast>) {
                self.count += 1;
                walk_index_expr(self, expr);
            }
        }

        let source = "void test() { int x = arr[0]; int y = matrix[1][2]; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = IndexCounter { count: 0 };
        walk_script(&mut counter, &script);

        // arr[0] = 1, matrix[1][2] = 2 (nested indexing counts both)
        assert_eq!(counter.count, 3);
    }

    #[test]
    fn test_member_expr_traversal() {
        struct MemberCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for MemberCounter {
            fn visit_member_expr(&mut self, expr: &MemberExpr<'src, 'ast>) {
                self.count += 1;
                walk_member_expr(self, expr);
            }
        }

        let source = "void test() { int x = obj.field; obj.method(); }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = MemberCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_postfix_expr_traversal() {
        struct PostfixCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for PostfixCounter {
            fn visit_postfix_expr(&mut self, expr: &PostfixExpr<'src, 'ast>) {
                self.count += 1;
                walk_postfix_expr(self, expr);
            }
        }

        let source = "void test() { int x = 0; x++; x--; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = PostfixCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_cast_expr_traversal() {
        struct CastCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for CastCounter {
            fn visit_cast_expr(&mut self, expr: &CastExpr<'src, 'ast>) {
                self.count += 1;
                walk_cast_expr(self, expr);
            }
        }

        let source = "void test() { float x = float(42); }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = CastCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_assign_expr_traversal() {
        struct AssignCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for AssignCounter {
            fn visit_assign_expr(&mut self, expr: &AssignExpr<'src, 'ast>) {
                self.count += 1;
                walk_assign_expr(self, expr);
            }
        }

        let source = "void test() { int x; x = 1; x += 2; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = AssignCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_lambda_expr_traversal() {
        struct LambdaCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for LambdaCounter {
            fn visit_lambda_expr(&mut self, expr: &LambdaExpr<'src, 'ast>) {
                self.count += 1;
                walk_lambda_expr(self, expr);
            }
        }

        let source = r#"
            funcdef void Callback();
            void test() {
                Callback @cb = function() { };
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = LambdaCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_init_list_expr_traversal() {
        struct InitListCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for InitListCounter {
            fn visit_init_list_expr(&mut self, expr: &InitListExpr<'src, 'ast>) {
                self.count += 1;
                walk_init_list_expr(self, expr);
            }
        }

        let source = "void test() { array<int> arr = {1, 2, 3}; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = InitListCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_paren_expr_traversal() {
        struct ParenCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for ParenCounter {
            fn visit_paren_expr(&mut self, expr: &ParenExpr<'src, 'ast>) {
                self.count += 1;
                walk_paren_expr(self, expr);
            }
        }

        let source = "void test() { int x = (1 + 2) * (3 + 4); }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = ParenCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    // ==================== Type Expression Tests ====================

    #[test]
    fn test_type_expr_traversal() {
        struct TypeExprCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for TypeExprCounter {
            fn visit_type_expr(&mut self, ty: &TypeExpr<'src, 'ast>) {
                self.count += 1;
                walk_type_expr(self, ty);
            }
        }

        let source = "void test() { int x; float y; array<int> arr; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = TypeExprCounter { count: 0 };
        walk_script(&mut counter, &script);

        // Return type (void) + 3 variable types + template arg (int)
        assert!(counter.count >= 4);
    }

    // ==================== Declaration Tests ====================

    #[test]
    fn test_typedef_traversal() {
        struct TypedefCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for TypedefCounter {
            fn visit_typedef_decl(&mut self, typedef: &TypedefDecl<'src, 'ast>) {
                self.count += 1;
                walk_typedef_decl(self, typedef);
            }
        }

        let source = "typedef int MyInt;";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = TypedefCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_funcdef_traversal() {
        struct FuncdefCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for FuncdefCounter {
            fn visit_funcdef_decl(&mut self, funcdef: &FuncdefDecl<'src, 'ast>) {
                self.count += 1;
                walk_funcdef_decl(self, funcdef);
            }
        }

        let source = "funcdef void Callback(int x, int y);";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = FuncdefCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_global_var_traversal() {
        struct GlobalVarCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for GlobalVarCounter {
            fn visit_global_var_decl(&mut self, var: &GlobalVarDecl<'src, 'ast>) {
                self.count += 1;
                walk_global_var_decl(self, var);
            }
        }

        let source = "int globalX = 10; float globalY;";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = GlobalVarCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_mixin_traversal() {
        struct MixinCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for MixinCounter {
            fn visit_mixin_decl(&mut self, mixin: &MixinDecl<'src, 'ast>) {
                self.count += 1;
                walk_mixin_decl(self, mixin);
            }
        }

        let source = "mixin class Helper { int value; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = MixinCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_import_traversal() {
        struct ImportCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for ImportCounter {
            fn visit_import_decl(&mut self, import: &ImportDecl<'src, 'ast>) {
                self.count += 1;
                walk_import_decl(self, import);
            }
        }

        let source = r#"import void external() from "module";"#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = ImportCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    // ==================== Complex Traversal Tests ====================

    #[test]
    fn test_complex_script_traversal() {
        let source = r#"
            namespace Game {
                interface IEntity {
                    void update();
                }

                class Player : IEntity {
                    int health = 100;
                    float speed;

                    void update() {
                        if (health > 0) {
                            for (int i = 0; i < 10; i++) {
                                speed = speed + 0.1f;
                            }
                        }
                    }

                    int getHealth() {
                        return health;
                    }
                }

                void main() {
                    Player p;
                    p.update();
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = NodeCounter::new();
        walk_script(&mut counter, &script);

        assert_eq!(counter.namespaces, 1);
        assert_eq!(counter.interfaces, 1);
        assert_eq!(counter.classes, 1);
        assert_eq!(counter.functions, 3); // update, getHealth, main
        assert_eq!(counter.fields, 2);
        assert!(counter.if_stmts >= 1);
        assert!(counter.for_stmts >= 1);
    }

    #[test]
    fn test_virtual_property_traversal() {
        struct VirtualPropertyCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for VirtualPropertyCounter {
            fn visit_virtual_property_decl(&mut self, prop: &VirtualPropertyDecl<'src, 'ast>) {
                self.count += 1;
                walk_virtual_property_decl(self, prop);
            }
        }

        let source = r#"
            class Test {
                int value { get { return 0; } set { } }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = VirtualPropertyCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_foreach_traversal() {
        struct ForeachCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for ForeachCounter {
            fn visit_foreach_stmt(&mut self, stmt: &ForeachStmt<'src, 'ast>) {
                self.count += 1;
                walk_foreach_stmt(self, stmt);
            }
        }

        let source = r#"
            void test() {
                array<int> arr;
                foreach (int x : arr) {
                    int y = x;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = ForeachCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 1);
    }

    #[test]
    fn test_break_continue_traversal() {
        struct BreakContinueCounter {
            breaks: usize,
            continues: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for BreakContinueCounter {
            fn visit_break_stmt(&mut self, stmt: &BreakStmt) {
                self.breaks += 1;
                // Note: break is a leaf node, no walk function needed
                let _ = stmt;
            }
            fn visit_continue_stmt(&mut self, stmt: &ContinueStmt) {
                self.continues += 1;
                let _ = stmt;
            }
        }

        let source = r#"
            void test() {
                for (int i = 0; i < 10; i++) {
                    if (i == 5) continue;
                    if (i == 8) break;
                }
            }
        "#;
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = BreakContinueCounter { breaks: 0, continues: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.breaks, 1);
        assert_eq!(counter.continues, 1);
    }

    #[test]
    fn test_var_decl_stmt_traversal() {
        struct VarDeclCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for VarDeclCounter {
            fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt<'src, 'ast>) {
                self.count += 1;
                walk_var_decl_stmt(self, stmt);
            }
        }

        let source = "void test() { int x = 1, y = 2; float z; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = VarDeclCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2); // int x, y and float z
    }

    #[test]
    fn test_return_type_traversal() {
        struct ReturnTypeCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for ReturnTypeCounter {
            fn visit_return_type(&mut self, ty: &ReturnType<'src, 'ast>) {
                self.count += 1;
                walk_return_type(self, ty);
            }
        }

        let source = "int foo() { return 0; } float bar() { return 0.0f; }";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = ReturnTypeCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 2);
    }

    #[test]
    fn test_param_type_traversal() {
        struct ParamTypeCounter {
            count: usize,
        }
        impl<'src, 'ast> Visitor<'src, 'ast> for ParamTypeCounter {
            fn visit_param_type(&mut self, ty: &ParamType<'src, 'ast>) {
                self.count += 1;
                walk_param_type(self, ty);
            }
        }

        let source = "void test(int a, float b, bool c) {}";
        let arena = bumpalo::Bump::new();
        let script = crate::ast::parse(source, &arena).expect("Failed to parse");

        let mut counter = ParamTypeCounter { count: 0 };
        walk_script(&mut counter, &script);

        assert_eq!(counter.count, 3);
    }
}