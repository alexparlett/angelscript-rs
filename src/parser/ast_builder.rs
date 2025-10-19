use crate::parser::ast::*;
use crate::parser::{AngelScriptParser, Rule};
use pest::iterators::Pair;
use pest::Parser;

pub(crate) fn build_script_item(pair: Pair<Rule>) -> Option<ScriptItem> {
    match pair.as_rule() {
        Rule::INCLUDE => {
            let string = pair.into_inner().next()?.as_str();
            Some(ScriptItem::Include(string.to_string()))
        }
        Rule::FUNC => Some(ScriptItem::Func(build_func(pair))),
        Rule::CLASS => Some(ScriptItem::Class(build_class(pair))),
        Rule::VAR => Some(ScriptItem::Var(build_var(pair))),
        Rule::ENUM => Some(ScriptItem::Enum(build_enum(pair))),
        Rule::NAMESPACE => Some(ScriptItem::Namespace(build_namespace(pair))),
        Rule::USING => Some(ScriptItem::Using(build_using(pair))),
        Rule::TYPEDEF => Some(ScriptItem::Typedef(build_typedef(pair))),
        Rule::INTERFACE => Some(ScriptItem::Interface(build_interface(pair))),
        Rule::FUNCDEF => Some(ScriptItem::FuncDef(build_funcdef(pair))),
        Rule::VIRTPROP => Some(ScriptItem::VirtProp(build_virtprop(pair))),
        Rule::IMPORT => Some(ScriptItem::Import(build_import(pair))),
        Rule::MIXIN => Some(ScriptItem::Mixin(build_mixin(pair))),
        _ => None,
    }
}

pub(crate) fn build_func(pair: Pair<Rule>) -> Func {
    let mut modifiers = Vec::new();
    let mut visibility = None;
    let mut return_type = None;
    let mut is_ref = false;
    let mut name = String::new();
    let mut params = Vec::new();
    let mut is_const = false;
    let mut attributes = Vec::new();
    let mut body = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::TYPE => return_type = Some(build_type(inner)),
            Rule::IDENTIFIER => name = inner.as_str().to_string(),
            Rule::PARAMLIST => params = build_paramlist(inner),
            Rule::FUNCATTR => attributes = build_funcattr(inner),
            Rule::STATBLOCK => body = Some(build_statblock(inner)),
            _ => {}
        }
    }

    Func {
        modifiers,
        visibility,
        return_type,
        is_ref,
        name,
        params,
        is_const,
        attributes,
        body,
    }
}

pub(crate) fn build_class(pair: Pair<Rule>) -> Class {
    let mut modifiers = Vec::new();
    let mut name = String::new();
    let mut extends = Vec::new();
    let mut members = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::IDENTIFIER => {
                if name.is_empty() {
                    name = inner.as_str().to_string();
                } else {
                    extends.push(inner.as_str().to_string());
                }
            }
            Rule::FUNC => members.push(ClassMember::Func(build_func(inner))),
            Rule::VAR => members.push(ClassMember::Var(build_var(inner))),
            Rule::VIRTPROP => members.push(ClassMember::VirtProp(build_virtprop(inner))),
            Rule::FUNCDEF => members.push(ClassMember::FuncDef(build_funcdef(inner))),
            _ => {}
        }
    }

    Class {
        modifiers,
        name,
        extends,
        members,
    }
}

pub(crate) fn build_var(pair: Pair<Rule>) -> Var {
    let mut visibility = None;
    let mut var_type = None;
    let mut declarations = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::TYPE => var_type = Some(build_type(inner)),
            Rule::IDENTIFIER => {
                declarations.push(VarDecl {
                    name: inner.as_str().to_string(),
                    initializer: None,
                });
            }
            _ => {}
        }
    }

    Var {
        visibility,
        var_type: var_type.unwrap_or_else(|| Type {
            is_const: false,
            scope: Scope { is_global: false, path: Vec::new() },
            datatype: DataType::Auto,
            template_types: Vec::new(),
            modifiers: Vec::new(),
        }),
        declarations,
    }
}

pub(crate) fn build_type(pair: Pair<Rule>) -> Type {
    let mut is_const = false;
    let mut scope = Scope {
        is_global: false,
        path: Vec::new(),
    };
    let mut datatype = DataType::Auto;
    let mut template_types = Vec::new();
    let mut modifiers = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::SCOPE => scope = build_scope(inner),
            Rule::DATATYPE => datatype = build_datatype(inner),
            Rule::TEMPLTYPELIST => template_types = build_templtypelist(inner),
            _ => {}
        }
    }

    Type {
        is_const,
        scope,
        datatype,
        template_types,
        modifiers,
    }
}

pub(crate) fn build_scope(pair: Pair<Rule>) -> Scope {
    let mut is_global = false;
    let mut path = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::IDENTIFIER {
            path.push(inner.as_str().to_string());
        }
    }

    Scope { is_global, path }
}

pub(crate) fn build_datatype(pair: Pair<Rule>) -> DataType {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::IDENTIFIER => DataType::Identifier(inner.as_str().to_string()),
        Rule::PRIMTYPE => DataType::PrimType(inner.as_str().to_string()),
        _ => DataType::Auto,
    }
}

pub(crate) fn build_templtypelist(pair: Pair<Rule>) -> Vec<Type> {
    pair.into_inner().map(|p| build_type(p)).collect()
}

pub(crate) fn build_paramlist(pair: Pair<Rule>) -> Vec<Param> {
    let mut params = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::TYPE {
            params.push(Param {
                param_type: build_type(inner),
                type_mod: None,
                name: None,
                default_value: None,
                is_variadic: false,
            });
        }
    }

    params
}

pub(crate) fn build_funcattr(pair: Pair<Rule>) -> Vec<String> {
    pair.into_inner()
        .map(|p| p.as_str().to_string())
        .collect()
}

pub(crate) fn build_statblock(pair: Pair<Rule>) -> StatBlock {
    let mut statements = Vec::new();

    for inner in pair.into_inner() {
        if let Some(stmt) = build_statement(inner) {
            statements.push(stmt);
        }
    }

    StatBlock { statements }
}

pub(crate) fn build_statement(pair: Pair<Rule>) -> Option<Statement> {
    match pair.as_rule() {
        Rule::RETURN => Some(Statement::Return(build_return(pair))),
        Rule::IF => Some(Statement::If(build_if(pair))),
        Rule::WHILE => Some(Statement::While(build_while(pair))),
        Rule::FOR => Some(Statement::For(build_for(pair))),
        Rule::BREAK => Some(Statement::Break),
        Rule::CONTINUE => Some(Statement::Continue),
        Rule::STATBLOCK => Some(Statement::Block(build_statblock(pair))),
        Rule::EXPRSTAT => Some(Statement::Expr(build_exprstat(pair))),
        Rule::VAR => Some(Statement::Var(build_var(pair))),
        Rule::SWITCH => Some(Statement::Switch(build_switch(pair))),
        Rule::DOWHILE => Some(Statement::DoWhile(build_dowhile(pair))),
        Rule::TRY => Some(Statement::Try(build_try(pair))),
        _ => None,
    }
}

pub(crate) fn build_return(pair: Pair<Rule>) -> ReturnStmt {
    let value = pair.into_inner().next().map(|p| build_expr(p));
    ReturnStmt { value }
}

pub(crate) fn build_if(pair: Pair<Rule>) -> IfStmt {
    let mut inner = pair.into_inner();
    let condition = build_expr(inner.next().unwrap());
    let then_branch = Box::new(build_statement(inner.next().unwrap()).unwrap());
    let else_branch = inner.next().map(|p| Box::new(build_statement(p).unwrap()));

    IfStmt {
        condition,
        then_branch,
        else_branch,
    }
}

pub(crate) fn build_while(pair: Pair<Rule>) -> WhileStmt {
    let mut inner = pair.into_inner();
    let condition = build_expr(inner.next().unwrap());
    let body = Box::new(build_statement(inner.next().unwrap()).unwrap());

    WhileStmt { condition, body }
}

pub(crate) fn build_dowhile(pair: Pair<Rule>) -> DoWhileStmt {
    let mut inner = pair.into_inner();
    let body = Box::new(build_statement(inner.next().unwrap()).unwrap());
    let condition = build_expr(inner.next().unwrap());

    DoWhileStmt { body, condition }
}

pub(crate) fn build_for(pair: Pair<Rule>) -> ForStmt {
    ForStmt {
        init: ForInit::Expr(None),
        condition: None,
        increment: Vec::new(),
        body: Box::new(Statement::Break),
    }
}

pub(crate) fn build_switch(pair: Pair<Rule>) -> SwitchStmt {
    let mut inner = pair.into_inner();
    let value = build_expr(inner.next().unwrap());
    let mut cases = Vec::new();

    for case_pair in inner {
        if case_pair.as_rule() == Rule::CASE {
            cases.push(build_case(case_pair));
        }
    }

    SwitchStmt { value, cases }
}

pub(crate) fn build_case(pair: Pair<Rule>) -> Case {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    let pattern = if first.as_str() == "default" {
        CasePattern::Default
    } else {
        CasePattern::Value(build_expr(first))
    };

    let mut statements = Vec::new();
    for stmt_pair in inner {
        if let Some(stmt) = build_statement(stmt_pair) {
            statements.push(stmt);
        }
    }

    Case { pattern, statements }
}

pub(crate) fn build_try(pair: Pair<Rule>) -> TryStmt {
    let mut inner = pair.into_inner();
    let try_block = build_statblock(inner.next().unwrap());
    let catch_block = build_statblock(inner.next().unwrap());

    TryStmt { try_block, catch_block }
}

pub(crate) fn build_exprstat(pair: Pair<Rule>) -> Option<Expr> {
    pair.into_inner().next().map(|p| build_expr(p))
}

pub(crate) fn build_expr(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::ASSIGN | Rule::CONDITION | Rule::EXPR => {
            let inner = pair.into_inner().next();
            if let Some(inner_pair) = inner {
                build_expr(inner_pair)
            } else {
                Expr::Void
            }
        }
        Rule::EXPRTERM => build_exprterm(pair),
        Rule::LITERAL => Expr::Literal(build_literal(pair)),
        _ => Expr::Void,
    }
}

pub(crate) fn build_exprterm(pair: Pair<Rule>) -> Expr {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::EXPRVALUE => build_exprvalue(inner),
        _ => Expr::Void,
    }
}

pub(crate) fn build_exprvalue(pair: Pair<Rule>) -> Expr {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::LITERAL => Expr::Literal(build_literal(inner)),
        Rule::VARACCESS => build_varaccess(inner),
        Rule::FUNCCALL => Expr::FuncCall(build_funccall(inner)),
        _ => Expr::Void,
    }
}

pub(crate) fn build_literal(pair: Pair<Rule>) -> Literal {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::NUMBER => Literal::Number(inner.as_str().to_string()),
        Rule::STRING => Literal::String(inner.as_str().to_string()),
        Rule::BITS => Literal::Bits(inner.as_str().to_string()),
        _ => match inner.as_str() {
            "true" => Literal::Bool(true),
            "false" => Literal::Bool(false),
            "null" => Literal::Null,
            _ => Literal::Null,
        },
    }
}

pub(crate) fn build_varaccess(pair: Pair<Rule>) -> Expr {
    let mut inner = pair.into_inner();
    let scope = build_scope(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    Expr::VarAccess(scope, name)
}

pub(crate) fn build_funccall(pair: Pair<Rule>) -> FuncCall {
    let mut scope = Scope {
        is_global: false,
        path: Vec::new(),
    };
    let mut name = String::new();
    let mut template_types = Vec::new();
    let mut args = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::SCOPE => scope = build_scope(inner),
            Rule::IDENTIFIER => name = inner.as_str().to_string(),
            Rule::TEMPLTYPELIST => template_types = build_templtypelist(inner),
            Rule::ARGLIST => args = build_arglist(inner),
            _ => {}
        }
    }

    FuncCall {
        scope,
        name,
        template_types,
        args,
    }
}

pub(crate) fn build_arglist(pair: Pair<Rule>) -> Vec<Arg> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::ASSIGN)
        .map(|p| Arg {
            name: None,
            value: build_expr(p),
        })
        .collect()
}

// Stub implementations for remaining builders
pub(crate) fn build_enum(_pair: Pair<Rule>) -> Enum {
    Enum {
        modifiers: Vec::new(),
        name: String::new(),
        variants: Vec::new(),
    }
}

pub(crate) fn build_namespace(_pair: Pair<Rule>) -> Namespace {
    Namespace {
        name: Vec::new(),
        items: Vec::new(),
    }
}

pub(crate) fn build_using(_pair: Pair<Rule>) -> Using {
    Using {
        namespace: Vec::new(),
    }
}

pub(crate) fn build_typedef(_pair: Pair<Rule>) -> Typedef {
    Typedef {
        prim_type: String::new(),
        name: String::new(),
    }
}

pub(crate) fn build_interface(_pair: Pair<Rule>) -> Interface {
    Interface {
        modifiers: Vec::new(),
        name: String::new(),
        extends: Vec::new(),
        members: Vec::new(),
    }
}

pub(crate) fn build_funcdef(_pair: Pair<Rule>) -> FuncDef {
    FuncDef {
        modifiers: Vec::new(),
        return_type: Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: Vec::new(),
            },
            datatype: DataType::Auto,
            template_types: Vec::new(),
            modifiers: Vec::new(),
        },
        is_ref: false,
        name: String::new(),
        params: Vec::new(),
    }
}

pub(crate) fn build_virtprop(_pair: Pair<Rule>) -> VirtProp {
    VirtProp {
        visibility: None,
        prop_type: Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: Vec::new(),
            },
            datatype: DataType::Auto,
            template_types: Vec::new(),
            modifiers: Vec::new(),
        },
        is_ref: false,
        name: String::new(),
        accessors: Vec::new(),
    }
}

pub(crate) fn build_import(_pair: Pair<Rule>) -> Import {
    Import {
        type_name: Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: Vec::new(),
            },
            datatype: DataType::Auto,
            template_types: Vec::new(),
            modifiers: Vec::new(),
        },
        is_ref: false,
        identifier: String::new(),
        params: Vec::new(),
        from: String::new(),
    }
}

pub(crate) fn build_mixin(_pair: Pair<Rule>) -> Mixin {
    Mixin {
        class: Class {
            modifiers: Vec::new(),
            name: String::new(),
            extends: Vec::new(),
            members: Vec::new(),
        },
    }
}
