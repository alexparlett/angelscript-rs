//! Visitor pattern for traversing the AST.
//!
//! This module provides a `Visitor` trait and corresponding `walk_*` functions
//! that enable easy traversal and analysis of AngelScript AST nodes.
//!
//! # Example: Using the Visitor Pattern
//!
//! ```
//! use angelscript::{parse_lenient, visitor::Visitor, FunctionDecl, Item, Script};
//!
//! struct FunctionCounter {
//!     count: usize,
//! }
//!
//! impl Visitor for FunctionCounter {
//!     // Override the function visit method
//! }
//!
//! let source = "void foo() {} void bar() {}";
//! let (script, _) = parse_lenient(source);
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
pub trait Visitor: Sized {
    // === Script and Items ===

    /// Visit a script (root node).
    fn visit_script(&mut self, script: &Script) {
        walk_script(self, script);
    }

    /// Visit a top-level item.
    fn visit_item(&mut self, item: &Item) {
        walk_item(self, item);
    }

    /// Visit a function declaration.
    fn visit_function_decl(&mut self, func: &FunctionDecl) {
        walk_function_decl(self, func);
    }

    /// Visit a class declaration.
    fn visit_class_decl(&mut self, class: &ClassDecl) {
        walk_class_decl(self, class);
    }

    /// Visit an interface declaration.
    fn visit_interface_decl(&mut self, interface: &InterfaceDecl) {
        walk_interface_decl(self, interface);
    }

    /// Visit an enum declaration.
    fn visit_enum_decl(&mut self, enum_decl: &EnumDecl) {
        walk_enum_decl(self, enum_decl);
    }

    /// Visit a global variable declaration.
    fn visit_global_var_decl(&mut self, var: &GlobalVarDecl) {
        walk_global_var_decl(self, var);
    }

    /// Visit a namespace declaration.
    fn visit_namespace_decl(&mut self, namespace: &NamespaceDecl) {
        walk_namespace_decl(self, namespace);
    }

    /// Visit a typedef declaration.
    fn visit_typedef_decl(&mut self, typedef: &TypedefDecl) {
        walk_typedef_decl(self, typedef);
    }

    /// Visit a funcdef declaration.
    fn visit_funcdef_decl(&mut self, funcdef: &FuncdefDecl) {
        walk_funcdef_decl(self, funcdef);
    }

    /// Visit a mixin declaration.
    fn visit_mixin_decl(&mut self, mixin: &MixinDecl) {
        walk_mixin_decl(self, mixin);
    }

    /// Visit an import declaration.
    fn visit_import_decl(&mut self, import: &ImportDecl) {
        walk_import_decl(self, import);
    }

    // === Class Members ===

    /// Visit a class member.
    fn visit_class_member(&mut self, member: &ClassMember) {
        walk_class_member(self, member);
    }

    /// Visit a field declaration.
    fn visit_field_decl(&mut self, field: &FieldDecl) {
        walk_field_decl(self, field);
    }

    /// Visit a virtual property declaration.
    fn visit_virtual_property_decl(&mut self, prop: &VirtualPropertyDecl) {
        walk_virtual_property_decl(self, prop);
    }

    /// Visit a property accessor.
    fn visit_property_accessor(&mut self, accessor: &PropertyAccessor) {
        walk_property_accessor(self, accessor);
    }

    // === Interface Members ===

    /// Visit an interface member.
    fn visit_interface_member(&mut self, member: &InterfaceMember) {
        walk_interface_member(self, member);
    }

    /// Visit an interface method.
    fn visit_interface_method(&mut self, method: &InterfaceMethod) {
        walk_interface_method(self, method);
    }

    // === Statements ===

    /// Visit a statement.
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }

    /// Visit an expression statement.
    fn visit_expr_stmt(&mut self, stmt: &ExprStmt) {
        walk_expr_stmt(self, stmt);
    }

    /// Visit a variable declaration statement.
    fn visit_var_decl_stmt(&mut self, stmt: &VarDeclStmt) {
        walk_var_decl_stmt(self, stmt);
    }

    /// Visit a return statement.
    fn visit_return_stmt(&mut self, stmt: &ReturnStmt) {
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
    fn visit_block(&mut self, block: &Block) {
        walk_block(self, block);
    }

    /// Visit an if statement.
    fn visit_if_stmt(&mut self, stmt: &IfStmt) {
        walk_if_stmt(self, stmt);
    }

    /// Visit a while statement.
    fn visit_while_stmt(&mut self, stmt: &WhileStmt) {
        walk_while_stmt(self, stmt);
    }

    /// Visit a do-while statement.
    fn visit_do_while_stmt(&mut self, stmt: &DoWhileStmt) {
        walk_do_while_stmt(self, stmt);
    }

    /// Visit a for statement.
    fn visit_for_stmt(&mut self, stmt: &ForStmt) {
        walk_for_stmt(self, stmt);
    }

    /// Visit a foreach statement.
    fn visit_foreach_stmt(&mut self, stmt: &ForeachStmt) {
        walk_foreach_stmt(self, stmt);
    }

    /// Visit a switch statement.
    fn visit_switch_stmt(&mut self, stmt: &SwitchStmt) {
        walk_switch_stmt(self, stmt);
    }

    /// Visit a try-catch statement.
    fn visit_try_catch_stmt(&mut self, stmt: &TryCatchStmt) {
        walk_try_catch_stmt(self, stmt);
    }

    // === Expressions ===

    /// Visit an expression.
    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }

    /// Visit a literal expression.
    fn visit_literal_expr(&mut self, _expr: &LiteralExpr) {
        // Leaf node, no children
    }

    /// Visit an identifier expression.
    fn visit_ident_expr(&mut self, _expr: &IdentExpr) {
        // Leaf node, no children
    }

    /// Visit a binary expression.
    fn visit_binary_expr(&mut self, expr: &BinaryExpr) {
        walk_binary_expr(self, expr);
    }

    /// Visit a unary expression.
    fn visit_unary_expr(&mut self, expr: &UnaryExpr) {
        walk_unary_expr(self, expr);
    }

    /// Visit an assignment expression.
    fn visit_assign_expr(&mut self, expr: &AssignExpr) {
        walk_assign_expr(self, expr);
    }

    /// Visit a ternary expression.
    fn visit_ternary_expr(&mut self, expr: &TernaryExpr) {
        walk_ternary_expr(self, expr);
    }

    /// Visit a call expression.
    fn visit_call_expr(&mut self, expr: &CallExpr) {
        walk_call_expr(self, expr);
    }

    /// Visit an index expression.
    fn visit_index_expr(&mut self, expr: &IndexExpr) {
        walk_index_expr(self, expr);
    }

    /// Visit a member expression.
    fn visit_member_expr(&mut self, expr: &MemberExpr) {
        walk_member_expr(self, expr);
    }

    /// Visit a postfix expression.
    fn visit_postfix_expr(&mut self, expr: &PostfixExpr) {
        walk_postfix_expr(self, expr);
    }

    /// Visit a cast expression.
    fn visit_cast_expr(&mut self, expr: &CastExpr) {
        walk_cast_expr(self, expr);
    }

    /// Visit a lambda expression.
    fn visit_lambda_expr(&mut self, expr: &LambdaExpr) {
        walk_lambda_expr(self, expr);
    }

    /// Visit an initializer list expression.
    fn visit_init_list_expr(&mut self, expr: &InitListExpr) {
        walk_init_list_expr(self, expr);
    }

    /// Visit a parenthesized expression.
    fn visit_paren_expr(&mut self, expr: &ParenExpr) {
        walk_paren_expr(self, expr);
    }

    // === Types ===

    /// Visit a type expression.
    fn visit_type_expr(&mut self, ty: &TypeExpr) {
        walk_type_expr(self, ty);
    }

    /// Visit a parameter type.
    fn visit_param_type(&mut self, ty: &ParamType) {
        walk_param_type(self, ty);
    }

    /// Visit a return type.
    fn visit_return_type(&mut self, ty: &ReturnType) {
        walk_return_type(self, ty);
    }
}

// === Walk Functions ===
// These provide default traversal logic for each node type.

/// Walk a script (root node).
pub fn walk_script<V: Visitor>(visitor: &mut V, script: &Script) {
    for item in script.items() {
        visitor.visit_item(item);
    }
}

/// Walk a top-level item.
pub fn walk_item<V: Visitor>(visitor: &mut V, item: &Item) {
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
pub fn walk_function_decl<V: Visitor>(visitor: &mut V, func: &FunctionDecl) {
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
pub fn walk_class_decl<V: Visitor>(visitor: &mut V, class: &ClassDecl) {
    for member in class.members {
        visitor.visit_class_member(member);
    }
}

/// Walk an interface declaration.
pub fn walk_interface_decl<V: Visitor>(visitor: &mut V, interface: &InterfaceDecl) {
    for member in interface.members {
        visitor.visit_interface_member(member);
    }
}

/// Walk an enum declaration.
pub fn walk_enum_decl<V: Visitor>(visitor: &mut V, enum_decl: &EnumDecl) {
    for enumerator in enum_decl.enumerators {
        if let Some(value) = &enumerator.value {
            visitor.visit_expr(value);
        }
    }
}

/// Walk a global variable declaration.
pub fn walk_global_var_decl<V: Visitor>(visitor: &mut V, var: &GlobalVarDecl) {
    visitor.visit_type_expr(&var.ty);
    if let Some(init) = &var.init {
        visitor.visit_expr(init);
    }
}

/// Walk a namespace declaration.
pub fn walk_namespace_decl<V: Visitor>(visitor: &mut V, namespace: &NamespaceDecl) {
    for item in namespace.items {
        visitor.visit_item(item);
    }
}

/// Walk a typedef declaration.
pub fn walk_typedef_decl<V: Visitor>(visitor: &mut V, typedef: &TypedefDecl) {
    visitor.visit_type_expr(&typedef.base_type);
}

/// Walk a funcdef declaration.
pub fn walk_funcdef_decl<V: Visitor>(visitor: &mut V, funcdef: &FuncdefDecl) {
    visitor.visit_return_type(&funcdef.return_type);
    for param in funcdef.params {
        visitor.visit_param_type(&param.ty);
        if let Some(default) = &param.default {
            visitor.visit_expr(default);
        }
    }
}

/// Walk a mixin declaration.
pub fn walk_mixin_decl<V: Visitor>(visitor: &mut V, mixin: &MixinDecl) {
    visitor.visit_class_decl(&mixin.class);
}

/// Walk an import declaration.
pub fn walk_import_decl<V: Visitor>(visitor: &mut V, import: &ImportDecl) {
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
pub fn walk_class_member<V: Visitor>(visitor: &mut V, member: &ClassMember) {
    match member {
        ClassMember::Method(method) => visitor.visit_function_decl(method),
        ClassMember::Field(field) => visitor.visit_field_decl(field),
        ClassMember::VirtualProperty(prop) => visitor.visit_virtual_property_decl(prop),
        ClassMember::Funcdef(funcdef) => visitor.visit_funcdef_decl(funcdef),
    }
}

/// Walk a field declaration.
pub fn walk_field_decl<V: Visitor>(visitor: &mut V, field: &FieldDecl) {
    visitor.visit_type_expr(&field.ty);
    if let Some(init) = &field.init {
        visitor.visit_expr(init);
    }
}

/// Walk a virtual property declaration.
pub fn walk_virtual_property_decl<V: Visitor>(visitor: &mut V, prop: &VirtualPropertyDecl) {
    visitor.visit_return_type(&prop.ty);
    for accessor in prop.accessors {
        visitor.visit_property_accessor(accessor);
    }
}

/// Walk a property accessor.
pub fn walk_property_accessor<V: Visitor>(visitor: &mut V, accessor: &PropertyAccessor) {
    if let Some(body) = &accessor.body {
        visitor.visit_block(body);
    }
}

// === Interface Members ===

/// Walk an interface member.
pub fn walk_interface_member<V: Visitor>(visitor: &mut V, member: &InterfaceMember) {
    match member {
        InterfaceMember::Method(method) => visitor.visit_interface_method(method),
        InterfaceMember::VirtualProperty(prop) => visitor.visit_virtual_property_decl(prop),
    }
}

/// Walk an interface method.
pub fn walk_interface_method<V: Visitor>(visitor: &mut V, method: &InterfaceMethod) {
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
pub fn walk_stmt<V: Visitor>(visitor: &mut V, stmt: &Stmt) {
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
pub fn walk_expr_stmt<V: Visitor>(visitor: &mut V, stmt: &ExprStmt) {
    if let Some(expr) = &stmt.expr {
        visitor.visit_expr(expr);
    }
}

/// Walk a variable declaration statement.
pub fn walk_var_decl_stmt<V: Visitor>(visitor: &mut V, stmt: &VarDeclStmt) {
    visitor.visit_type_expr(&stmt.ty);
    for var in stmt.vars {
        if let Some(init) = &var.init {
            visitor.visit_expr(init);
        }
    }
}

/// Walk a return statement.
pub fn walk_return_stmt<V: Visitor>(visitor: &mut V, stmt: &ReturnStmt) {
    if let Some(value) = &stmt.value {
        visitor.visit_expr(value);
    }
}

/// Walk a block.
pub fn walk_block<V: Visitor>(visitor: &mut V, block: &Block) {
    for stmt in block.stmts {
        visitor.visit_stmt(stmt);
    }
}

/// Walk an if statement.
pub fn walk_if_stmt<V: Visitor>(visitor: &mut V, stmt: &IfStmt) {
    visitor.visit_expr(&stmt.condition);
    visitor.visit_stmt(&stmt.then_stmt);
    if let Some(else_stmt) = &stmt.else_stmt {
        visitor.visit_stmt(else_stmt);
    }
}

/// Walk a while statement.
pub fn walk_while_stmt<V: Visitor>(visitor: &mut V, stmt: &WhileStmt) {
    visitor.visit_expr(&stmt.condition);
    visitor.visit_stmt(&stmt.body);
}

/// Walk a do-while statement.
pub fn walk_do_while_stmt<V: Visitor>(visitor: &mut V, stmt: &DoWhileStmt) {
    visitor.visit_stmt(&stmt.body);
    visitor.visit_expr(&stmt.condition);
}

/// Walk a for statement.
pub fn walk_for_stmt<V: Visitor>(visitor: &mut V, stmt: &ForStmt) {
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
    visitor.visit_stmt(&stmt.body);
}

/// Walk a foreach statement.
pub fn walk_foreach_stmt<V: Visitor>(visitor: &mut V, stmt: &ForeachStmt) {
    // Visit variable types
    for var in stmt.vars {
        visitor.visit_type_expr(&var.ty);
    }

    // Visit iterable expression
    visitor.visit_expr(&stmt.expr);

    // Visit body
    visitor.visit_stmt(&stmt.body);
}

/// Walk a switch statement.
pub fn walk_switch_stmt<V: Visitor>(visitor: &mut V, stmt: &SwitchStmt) {
    visitor.visit_expr(&stmt.expr);
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
pub fn walk_try_catch_stmt<V: Visitor>(visitor: &mut V, stmt: &TryCatchStmt) {
    visitor.visit_block(&stmt.try_block);
    visitor.visit_block(&stmt.catch_block);
}

// === Expressions ===

/// Walk an expression.
pub fn walk_expr<V: Visitor>(visitor: &mut V, expr: &Expr) {
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
pub fn walk_binary_expr<V: Visitor>(visitor: &mut V, expr: &BinaryExpr) {
    visitor.visit_expr(&expr.left);
    visitor.visit_expr(&expr.right);
}

/// Walk a unary expression.
pub fn walk_unary_expr<V: Visitor>(visitor: &mut V, expr: &UnaryExpr) {
    visitor.visit_expr(&expr.operand);
}

/// Walk an assignment expression.
pub fn walk_assign_expr<V: Visitor>(visitor: &mut V, expr: &AssignExpr) {
    visitor.visit_expr(&expr.target);
    visitor.visit_expr(&expr.value);
}

/// Walk a ternary expression.
pub fn walk_ternary_expr<V: Visitor>(visitor: &mut V, expr: &TernaryExpr) {
    visitor.visit_expr(&expr.condition);
    visitor.visit_expr(&expr.then_expr);
    visitor.visit_expr(&expr.else_expr);
}

/// Walk a call expression.
pub fn walk_call_expr<V: Visitor>(visitor: &mut V, expr: &CallExpr) {
    visitor.visit_expr(&expr.callee);
    for arg in expr.args {
        visitor.visit_expr(&arg.value);
    }
}

/// Walk an index expression.
pub fn walk_index_expr<V: Visitor>(visitor: &mut V, expr: &IndexExpr) {
    visitor.visit_expr(&expr.object);
    for index in expr.indices {
        visitor.visit_expr(&index.index);
    }
}

/// Walk a member expression.
pub fn walk_member_expr<V: Visitor>(visitor: &mut V, expr: &MemberExpr) {
    visitor.visit_expr(&expr.object);
    if let MemberAccess::Method { args, .. } = &expr.member {
        for arg in *args {
            visitor.visit_expr(&arg.value);
        }
    }
}

/// Walk a postfix expression.
pub fn walk_postfix_expr<V: Visitor>(visitor: &mut V, expr: &PostfixExpr) {
    visitor.visit_expr(&expr.operand);
}

/// Walk a cast expression.
pub fn walk_cast_expr<V: Visitor>(visitor: &mut V, expr: &CastExpr) {
    visitor.visit_type_expr(&expr.target_type);
    visitor.visit_expr(&expr.expr);
}

/// Walk a lambda expression.
pub fn walk_lambda_expr<V: Visitor>(visitor: &mut V, expr: &LambdaExpr) {
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
    visitor.visit_block(&expr.body);
}

/// Walk an initializer list expression.
pub fn walk_init_list_expr<V: Visitor>(visitor: &mut V, expr: &InitListExpr) {
    if let Some(ty) = &expr.ty {
        visitor.visit_type_expr(ty);
    }
    for element in expr.elements {
        match element {
            InitElement::Expr(e) => visitor.visit_expr(e),
            InitElement::InitList(list) => visitor.visit_init_list_expr(&list),
        }
    }
}

/// Walk a parenthesized expression.
pub fn walk_paren_expr<V: Visitor>(visitor: &mut V, expr: &ParenExpr) {
    visitor.visit_expr(&expr.expr);
}

// === Types ===

/// Walk a type expression.
pub fn walk_type_expr<V: Visitor>(visitor: &mut V, ty: &TypeExpr) {
    // Visit template arguments
    for arg in ty.template_args {
        visitor.visit_type_expr(arg);
    }
}

/// Walk a parameter type.
pub fn walk_param_type<V: Visitor>(visitor: &mut V, ty: &ParamType) {
    visitor.visit_type_expr(&ty.ty);
}

/// Walk a return type.
pub fn walk_return_type<V: Visitor>(visitor: &mut V, ty: &ReturnType) {
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

    impl Visitor for FunctionCounter {
        fn visit_function_decl(&mut self, func: &FunctionDecl) {
            self.count += 1;
            // Continue walking to count nested functions
            walk_function_decl(self, func);
        }
    }

    /// Example visitor that collects all identifier names.
    struct IdentCollector {
        names: Vec<String>,
    }

    impl Visitor for IdentCollector {
        fn visit_ident_expr(&mut self, expr: &IdentExpr) {
            self.names.push(expr.ident.name.to_string());
        }
    }

    #[test]
    fn test_function_counter() {
        // Parse a simple script with two functions
        let source = "void foo() {} int bar() { return 0; }";
        let script = crate::parse(source).expect("Failed to parse test script");

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

        impl Visitor for CustomVisitor {
            fn visit_return_stmt(&mut self, _stmt: &ReturnStmt) {
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
}