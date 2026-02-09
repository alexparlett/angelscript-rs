//! Recursive descent parser for AngelScript.
//!
//! Produces a typed AST from a token stream. Uses Pratt parsing for
//! expression precedence.

use angelscript_core::error::{Diagnostic, Severity, SourceLocation};
use angelscript_lexer::{
    lexer::Lexer,
    token::{Span, Token, TokenKind},
};

use crate::ast::*;

// ── Parser ─────────────────────────────────────────────────────────────────

pub struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let tokens: Vec<Token> = Lexer::tokenize_all(source)
            .into_iter()
            .filter(|t| !t.kind.is_trivia())
            .collect();
        Parser {
            source,
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    // ── Token access ───────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream should have at least EOF")
        })
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn at_eof(&self) -> bool {
        self.at(TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Token {
        if self.at(kind) {
            self.advance()
        } else {
            self.error(format!("expected '{}', found '{}'", kind, self.peek_kind()));
            // Return a synthetic token at current position
            Token::new(kind, self.current_span())
        }
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn text(&self, token: &Token) -> &'a str {
        token.text(self.source)
    }

    fn current_span(&self) -> Span {
        self.peek().span
    }

    fn save_pos(&self) -> usize {
        self.pos
    }

    fn restore_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn span_from(&self, start: Span) -> Span {
        if self.pos > 0 {
            start.merge(self.tokens[self.pos - 1].span)
        } else {
            start
        }
    }

    /// Check if the current identifier has the given text.
    fn identifier_is(&self, text: &str) -> bool {
        self.at(TokenKind::Identifier) && self.text(self.peek()) == text
    }

    fn peek_ahead(&self, offset: usize) -> TokenKind {
        self.tokens
            .get(self.pos + offset)
            .map(|t| t.kind)
            .unwrap_or(TokenKind::Eof)
    }

    // ── Error reporting ────────────────────────────────────────────────

    fn error(&mut self, msg: String) {
        let span = self.current_span();
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: msg,
            location: Some(SourceLocation::from_offset(self.source, span.start)),
        });
    }

    fn error_at(&mut self, span: Span, msg: String) {
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: msg,
            location: Some(SourceLocation::from_offset(self.source, span.start)),
        });
    }

    /// Skip tokens until we reach a synchronization point.
    fn synchronize(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Semicolon => {
                    self.advance();
                    break;
                }
                TokenKind::CloseBrace => break,
                TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Enum
                | TokenKind::Funcdef
                | TokenKind::Typedef
                | TokenKind::Namespace
                | TokenKind::Import => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Entry point ────────────────────────────────────────────────────

    /// Parse a complete script.
    pub fn parse_script(&mut self) -> Script {
        let start = self.current_span();
        let mut declarations = Vec::new();

        while !self.at_eof() {
            if self.eat(TokenKind::Semicolon) {
                continue;
            }
            match self.try_parse_declaration() {
                Some(decl) => declarations.push(decl),
                None => {
                    self.error(format!(
                        "unexpected token '{}' at top level",
                        self.peek_kind()
                    ));
                    let before = self.pos;
                    self.synchronize();
                    // Ensure progress — if synchronize didn't advance, skip one token
                    if self.pos == before {
                        self.advance();
                    }
                }
            }
        }

        Script {
            declarations,
            span: self.span_from(start),
        }
    }

    // ── Top-level declarations ─────────────────────────────────────────

    fn try_parse_declaration(&mut self) -> Option<Declaration> {
        // Check for context keywords that start declarations
        if self.identifier_is("shared") || self.identifier_is("external") {
            return self.parse_flagged_declaration();
        }
        if self.identifier_is("abstract") || self.identifier_is("final") {
            return self.parse_flagged_declaration();
        }
        if self.identifier_is("mixin") {
            return Some(self.parse_mixin());
        }
        if self.identifier_is("using") {
            return Some(self.parse_using_namespace());
        }

        match self.peek_kind() {
            TokenKind::Class => Some(self.parse_class()),
            TokenKind::Interface => Some(self.parse_interface()),
            TokenKind::Enum => Some(self.parse_enum()),
            TokenKind::Typedef => Some(self.parse_typedef()),
            TokenKind::Funcdef => Some(self.parse_funcdef()),
            TokenKind::Namespace => Some(self.parse_namespace()),
            TokenKind::Import => Some(self.parse_import()),
            TokenKind::Private | TokenKind::Protected => {
                let access = self.parse_access_modifier();
                // After access modifier: could be function, variable, or virtual prop
                self.parse_member_with_access(access, true)
            }
            _ => {
                // Could be a function or variable declaration
                if self.is_func_decl() {
                    Some(Declaration::Function(self.parse_function_decl(None)))
                } else if self.is_var_decl() {
                    Some(Declaration::GlobalVar(self.parse_var_decl(None)))
                } else if self.is_virtual_property_decl() {
                    Some(Declaration::VirtualProperty(
                        self.parse_virtual_property(None),
                    ))
                } else {
                    None
                }
            }
        }
    }

    fn parse_flagged_declaration(&mut self) -> Option<Declaration> {
        // Collect flags: shared, external, abstract, final
        let mut class_flags = Vec::new();
        let mut enum_flags = Vec::new();
        let mut interface_flags = Vec::new();
        let mut funcdef_flags = Vec::new();
        let start = self.current_span();

        loop {
            if self.identifier_is("shared") {
                self.advance();
                class_flags.push(ClassFlag::Shared);
                enum_flags.push(EnumFlag::Shared);
                interface_flags.push(InterfaceFlag::Shared);
                funcdef_flags.push(FuncdefFlag::Shared);
            } else if self.identifier_is("external") {
                self.advance();
                class_flags.push(ClassFlag::External);
                enum_flags.push(EnumFlag::External);
                interface_flags.push(InterfaceFlag::External);
                funcdef_flags.push(FuncdefFlag::External);
            } else if self.identifier_is("abstract") {
                self.advance();
                class_flags.push(ClassFlag::Abstract);
            } else if self.identifier_is("final") {
                self.advance();
                class_flags.push(ClassFlag::Final);
            } else {
                break;
            }
        }

        match self.peek_kind() {
            TokenKind::Class => {
                let mut decl = self.parse_class_inner();
                decl.flags = class_flags;
                decl.span = self.span_from(start);
                Some(Declaration::Class(decl))
            }
            TokenKind::Interface => {
                let mut decl = self.parse_interface_inner();
                decl.flags = interface_flags;
                decl.span = self.span_from(start);
                Some(Declaration::Interface(decl))
            }
            TokenKind::Enum => {
                let mut decl = self.parse_enum_inner();
                decl.flags = enum_flags;
                decl.span = self.span_from(start);
                Some(Declaration::Enum(decl))
            }
            TokenKind::Funcdef => {
                let mut decl = self.parse_funcdef_inner();
                decl.flags = funcdef_flags;
                decl.span = self.span_from(start);
                Some(Declaration::Funcdef(decl))
            }
            _ => {
                // Might be a function with 'shared' or 'external' in front
                // For now treat as error
                self.error_at(start, "expected declaration after modifier".to_string());
                None
            }
        }
    }

    // ── Class ──────────────────────────────────────────────────────────

    fn parse_class(&mut self) -> Declaration {
        Declaration::Class(self.parse_class_inner())
    }

    fn parse_class_inner(&mut self) -> ClassDecl {
        let start = self.current_span();
        self.expect(TokenKind::Class);
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();

        // Forward declaration?
        if self.eat(TokenKind::Semicolon) {
            return ClassDecl {
                name,
                flags: Vec::new(),
                base_classes: Vec::new(),
                members: Vec::new(),
                span: self.span_from(start),
            };
        }

        // Inheritance
        let mut bases = Vec::new();
        if self.eat(TokenKind::Colon) {
            bases.push(self.parse_scoped_identifier());
            while self.eat(TokenKind::Comma) {
                bases.push(self.parse_scoped_identifier());
            }
        }

        self.expect(TokenKind::OpenBrace);
        let mut members = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            if self.eat(TokenKind::Semicolon) {
                continue;
            }
            match self.try_parse_class_member() {
                Some(m) => members.push(m),
                None => {
                    self.error(format!(
                        "unexpected token '{}' in class body",
                        self.peek_kind()
                    ));
                    let before = self.pos;
                    self.synchronize();
                    if self.pos == before {
                        self.advance();
                    }
                }
            }
        }
        self.expect(TokenKind::CloseBrace);

        ClassDecl {
            name,
            flags: Vec::new(),
            base_classes: bases,
            members,
            span: self.span_from(start),
        }
    }

    fn try_parse_class_member(&mut self) -> Option<ClassMember> {
        // Access modifier
        let access = if self.at(TokenKind::Private) || self.at(TokenKind::Protected) {
            Some(self.parse_access_modifier())
        } else {
            None
        };

        // Funcdef inside class
        if self.at(TokenKind::Funcdef) {
            return Some(ClassMember::Funcdef(self.parse_funcdef_inner()));
        }

        // Destructor: ~ClassName()
        if self.at(TokenKind::Tilde) {
            return Some(ClassMember::Function(self.parse_destructor(access)));
        }

        if self.is_func_decl() {
            let f = self.parse_function_decl(access);
            // Mark constructor if no return type and name matches class context
            if f.return_type.is_none() {
                // Constructors have no return type — name checked by compiler
            }
            return Some(ClassMember::Function(f));
        }

        if self.is_virtual_property_decl() {
            return Some(ClassMember::VirtualProperty(
                self.parse_virtual_property(access),
            ));
        }

        if self.is_var_decl() {
            return Some(ClassMember::Variable(self.parse_var_decl(access)));
        }

        None
    }

    fn parse_destructor(&mut self, access: Option<AccessModifier>) -> FunctionDecl {
        let start = self.current_span();
        self.expect(TokenKind::Tilde);
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();
        self.expect(TokenKind::OpenParen);
        self.expect(TokenKind::CloseParen);

        let body = if self.at(TokenKind::OpenBrace) {
            Some(Box::new(self.parse_statement_block()))
        } else {
            self.expect(TokenKind::Semicolon);
            None
        };

        FunctionDecl {
            return_type: None,
            name,
            params: Vec::new(),
            is_const: false,
            attributes: Vec::new(),
            access,
            body,
            is_destructor: true,
            span: self.span_from(start),
        }
    }

    // ── Interface ──────────────────────────────────────────────────────

    fn parse_interface(&mut self) -> Declaration {
        Declaration::Interface(self.parse_interface_inner())
    }

    fn parse_interface_inner(&mut self) -> InterfaceDecl {
        let start = self.current_span();
        self.expect(TokenKind::Interface);
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();

        if self.eat(TokenKind::Semicolon) {
            return InterfaceDecl {
                name,
                flags: Vec::new(),
                bases: Vec::new(),
                methods: Vec::new(),
                span: self.span_from(start),
            };
        }

        let mut bases = Vec::new();
        if self.eat(TokenKind::Colon) {
            bases.push(self.parse_scoped_identifier());
            while self.eat(TokenKind::Comma) {
                bases.push(self.parse_scoped_identifier());
            }
        }

        self.expect(TokenKind::OpenBrace);
        let mut methods = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            if self.eat(TokenKind::Semicolon) {
                continue;
            }
            methods.push(self.parse_interface_method());
        }
        self.expect(TokenKind::CloseBrace);

        InterfaceDecl {
            name,
            flags: Vec::new(),
            bases,
            methods,
            span: self.span_from(start),
        }
    }

    fn parse_interface_method(&mut self) -> InterfaceMethod {
        let start = self.current_span();
        let ret = self.parse_type_expr();
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();
        let params = self.parse_param_list();
        let is_const = self.eat(TokenKind::Const);
        self.expect(TokenKind::Semicolon);

        InterfaceMethod {
            return_type: ret,
            name,
            params,
            is_const,
            span: self.span_from(start),
        }
    }

    // ── Enum ───────────────────────────────────────────────────────────

    fn parse_enum(&mut self) -> Declaration {
        Declaration::Enum(self.parse_enum_inner())
    }

    fn parse_enum_inner(&mut self) -> EnumDecl {
        let start = self.current_span();
        self.expect(TokenKind::Enum);
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();

        if self.eat(TokenKind::Semicolon) {
            return EnumDecl {
                name,
                flags: Vec::new(),
                values: Vec::new(),
                span: self.span_from(start),
            };
        }

        self.expect(TokenKind::OpenBrace);
        let mut values = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            let v_start = self.current_span();
            let v_tok = self.expect(TokenKind::Identifier);
            let v_name = self.text(&v_tok).to_string();
            let value = if self.eat(TokenKind::Assign) {
                Some(self.parse_expr())
            } else {
                None
            };
            values.push(EnumValue {
                name: v_name,
                value,
                span: self.span_from(v_start),
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseBrace);

        EnumDecl {
            name,
            flags: Vec::new(),
            values,
            span: self.span_from(start),
        }
    }

    // ── Typedef ────────────────────────────────────────────────────────

    fn parse_typedef(&mut self) -> Declaration {
        let start = self.current_span();
        self.expect(TokenKind::Typedef);
        let prim = self.parse_type_expr();
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();
        self.expect(TokenKind::Semicolon);

        Declaration::Typedef(TypedefDecl {
            primitive: prim,
            name,
            span: self.span_from(start),
        })
    }

    // ── Funcdef ────────────────────────────────────────────────────────

    fn parse_funcdef(&mut self) -> Declaration {
        Declaration::Funcdef(self.parse_funcdef_inner())
    }

    fn parse_funcdef_inner(&mut self) -> FuncdefDecl {
        let start = self.current_span();
        self.expect(TokenKind::Funcdef);
        let ret = self.parse_type_expr();
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();
        let params = self.parse_param_list();
        self.expect(TokenKind::Semicolon);

        FuncdefDecl {
            flags: Vec::new(),
            return_type: ret,
            name,
            params,
            span: self.span_from(start),
        }
    }

    // ── Namespace ──────────────────────────────────────────────────────

    fn parse_namespace(&mut self) -> Declaration {
        let start = self.current_span();
        self.expect(TokenKind::Namespace);
        let mut names = Vec::new();
        let name_tok = self.expect(TokenKind::Identifier);
        names.push(self.text(&name_tok).to_string());
        while self.eat(TokenKind::Scope) {
            let name_tok = self.expect(TokenKind::Identifier);
            names.push(self.text(&name_tok).to_string());
        }

        self.expect(TokenKind::OpenBrace);
        let mut declarations = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            if self.eat(TokenKind::Semicolon) {
                continue;
            }
            match self.try_parse_declaration() {
                Some(decl) => declarations.push(decl),
                None => {
                    self.error(format!(
                        "unexpected token '{}' in namespace body",
                        self.peek_kind()
                    ));
                    let before = self.pos;
                    self.synchronize();
                    if self.pos == before {
                        self.advance();
                    }
                }
            }
        }
        self.expect(TokenKind::CloseBrace);

        Declaration::Namespace(NamespaceDecl {
            name: names,
            declarations,
            span: self.span_from(start),
        })
    }

    // ── Import ─────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> Declaration {
        let start = self.current_span();
        self.expect(TokenKind::Import);
        let ret = self.parse_type_expr();
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();
        let params = self.parse_param_list();
        let attrs = self.parse_func_attrs();

        // 'from' is a context keyword
        if self.identifier_is("from") {
            self.advance();
        } else {
            self.error("expected 'from' in import declaration".to_string());
        }

        let module_tok = self.expect(TokenKind::StringConstant);
        let module = self.parse_string_content(&module_tok);
        self.expect(TokenKind::Semicolon);

        Declaration::Import(ImportDecl {
            return_type: ret,
            name,
            params,
            attributes: attrs,
            module,
            span: self.span_from(start),
        })
    }

    // ── Mixin ──────────────────────────────────────────────────────────

    fn parse_mixin(&mut self) -> Declaration {
        let start = self.current_span();
        self.advance(); // 'mixin'
        let decl = self.parse_class_inner();
        Declaration::Mixin(ClassDecl {
            span: self.span_from(start),
            ..decl
        })
    }

    // ── Using namespace ────────────────────────────────────────────────

    fn parse_using_namespace(&mut self) -> Declaration {
        let start = self.current_span();
        self.advance(); // 'using'

        // Expect 'namespace' keyword — but it's a context keyword parsed as identifier
        if self.at(TokenKind::Namespace) {
            self.advance();
        } else {
            self.error("expected 'namespace' after 'using'".to_string());
        }

        let mut ns = Vec::new();
        let tok = self.expect(TokenKind::Identifier);
        ns.push(self.text(&tok).to_string());
        while self.eat(TokenKind::Scope) {
            let tok = self.expect(TokenKind::Identifier);
            ns.push(self.text(&tok).to_string());
        }
        self.expect(TokenKind::Semicolon);

        Declaration::UsingNamespace(UsingNamespaceDecl {
            namespace: ns,
            span: self.span_from(start),
        })
    }

    // ── Virtual property ───────────────────────────────────────────────

    fn parse_virtual_property(&mut self, access: Option<AccessModifier>) -> VirtualPropDecl {
        let start = self.current_span();
        let type_expr = self.parse_type_expr();
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();

        self.expect(TokenKind::OpenBrace);
        let mut accessors = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            let acc_start = self.current_span();
            let kind = if self.identifier_is("get") {
                self.advance();
                PropAccessorKind::Get
            } else if self.identifier_is("set") {
                self.advance();
                PropAccessorKind::Set
            } else {
                self.error("expected 'get' or 'set' in virtual property".to_string());
                self.synchronize();
                continue;
            };

            let is_const = self.eat(TokenKind::Const);
            let attrs = self.parse_func_attrs();
            let body = if self.at(TokenKind::OpenBrace) {
                Some(Box::new(self.parse_statement_block()))
            } else {
                self.expect(TokenKind::Semicolon);
                None
            };

            accessors.push(PropAccessor {
                kind,
                is_const,
                attributes: attrs,
                body,
                span: self.span_from(acc_start),
            });
        }
        self.expect(TokenKind::CloseBrace);

        VirtualPropDecl {
            type_expr,
            name,
            access,
            accessors,
            span: self.span_from(start),
        }
    }

    // ── Function declaration ───────────────────────────────────────────

    fn parse_function_decl(&mut self, access: Option<AccessModifier>) -> FunctionDecl {
        let start = self.current_span();

        // Return type (may be absent for constructors)
        let (return_type, name) = self.parse_function_head();

        let params = self.parse_param_list();
        let is_const = self.eat(TokenKind::Const);
        let attrs = self.parse_func_attrs();

        let body = if self.at(TokenKind::OpenBrace) {
            Some(Box::new(self.parse_statement_block()))
        } else {
            self.expect(TokenKind::Semicolon);
            None
        };

        FunctionDecl {
            return_type,
            name,
            params,
            is_const,
            attributes: attrs,
            access,
            body,
            is_destructor: false,
            span: self.span_from(start),
        }
    }

    /// Parse function return type and name.
    /// Constructors have no return type; regular functions have `TYPE NAME`.
    fn parse_function_head(&mut self) -> (Option<TypeExpr>, String) {
        // Try to parse as TYPE NAME(
        // If current token is identifier and next is '(', it's a constructor
        let saved = self.save_pos();

        // Try parsing a type first
        if self.can_start_type() {
            let ty = self.parse_type_expr();
            if self.at(TokenKind::Identifier) {
                let name_tok = self.advance();
                let name = self.text(&name_tok).to_string();
                return (Some(ty), name);
            }
            // No identifier after type — might be constructor
            self.restore_pos(saved);
        }

        // Constructor: just a name
        let name_tok = self.expect(TokenKind::Identifier);
        let name = self.text(&name_tok).to_string();
        (None, name)
    }

    fn parse_func_attrs(&mut self) -> Vec<FuncAttr> {
        let mut attrs = Vec::new();
        loop {
            if self.identifier_is("override") {
                self.advance();
                attrs.push(FuncAttr::Override);
            } else if self.identifier_is("final") {
                self.advance();
                attrs.push(FuncAttr::Final);
            } else if self.identifier_is("explicit") {
                self.advance();
                attrs.push(FuncAttr::Explicit);
            } else if self.identifier_is("property") {
                self.advance();
                attrs.push(FuncAttr::Property);
            } else if self.identifier_is("delete") {
                self.advance();
                attrs.push(FuncAttr::Delete);
            } else {
                break;
            }
        }
        attrs
    }

    fn parse_access_modifier(&mut self) -> AccessModifier {
        match self.peek_kind() {
            TokenKind::Private => {
                self.advance();
                AccessModifier::Private
            }
            TokenKind::Protected => {
                self.advance();
                AccessModifier::Protected
            }
            _ => {
                self.error("expected access modifier".to_string());
                AccessModifier::Private
            }
        }
    }

    fn parse_member_with_access(
        &mut self,
        access: AccessModifier,
        _is_global: bool,
    ) -> Option<Declaration> {
        if self.is_func_decl() {
            Some(Declaration::Function(
                self.parse_function_decl(Some(access)),
            ))
        } else if self.is_virtual_property_decl() {
            Some(Declaration::VirtualProperty(
                self.parse_virtual_property(Some(access)),
            ))
        } else if self.is_var_decl() {
            Some(Declaration::GlobalVar(self.parse_var_decl(Some(access))))
        } else {
            self.error("expected declaration after access modifier".to_string());
            self.synchronize();
            None
        }
    }

    // ── Parameter list ─────────────────────────────────────────────────

    fn parse_param_list(&mut self) -> Vec<Param> {
        self.expect(TokenKind::OpenParen);
        let mut params = Vec::new();

        // `void` in param list means no params
        if self.at(TokenKind::Void) && self.peek_ahead(1) == TokenKind::CloseParen {
            self.advance(); // consume void
            self.expect(TokenKind::CloseParen);
            return params;
        }

        while !self.at(TokenKind::CloseParen) && !self.at_eof() {
            let p_start = self.current_span();
            let type_expr = self.parse_type_expr();

            // Optional name
            let name = if self.at(TokenKind::Identifier)
                && !self.at(TokenKind::Comma)
                && !self.at(TokenKind::CloseParen)
            {
                let tok = self.advance();
                Some(self.text(&tok).to_string())
            } else {
                None
            };

            // Optional default
            let default = if self.eat(TokenKind::Assign) {
                Some(self.parse_assignment_expr())
            } else {
                None
            };

            params.push(Param {
                type_expr,
                name,
                default,
                span: self.span_from(p_start),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseParen);
        params
    }

    // ── Variable declarations ──────────────────────────────────────────

    fn parse_var_decl(&mut self, access: Option<AccessModifier>) -> VarDecl {
        let start = self.current_span();
        let type_expr = self.parse_type_expr();

        let mut declarators = Vec::new();
        loop {
            let d_start = self.current_span();
            let name_tok = self.expect(TokenKind::Identifier);
            let name = self.text(&name_tok).to_string();

            let init = if self.eat(TokenKind::Assign) {
                if self.at(TokenKind::OpenBrace) {
                    Some(VarInit::InitList(self.parse_init_list()))
                } else {
                    Some(VarInit::Expr(self.parse_assignment_expr()))
                }
            } else if self.at(TokenKind::OpenParen) {
                Some(VarInit::ConstructorArgs(self.parse_arg_values()))
            } else {
                None
            };

            declarators.push(VarDeclarator {
                name,
                init,
                span: self.span_from(d_start),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Semicolon);

        VarDecl {
            access,
            type_expr,
            declarators,
            span: self.span_from(start),
        }
    }

    fn parse_arg_values(&mut self) -> Vec<Expr> {
        self.expect(TokenKind::OpenParen);
        let mut args = Vec::new();
        while !self.at(TokenKind::CloseParen) && !self.at_eof() {
            args.push(self.parse_assignment_expr());
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseParen);
        args
    }

    // ── Init list ──────────────────────────────────────────────────────

    fn parse_init_list(&mut self) -> InitList {
        let start = self.current_span();
        self.expect(TokenKind::OpenBrace);
        let mut items = Vec::new();

        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            if self.at(TokenKind::OpenBrace) {
                items.push(InitListItem::Nested(self.parse_init_list()));
            } else {
                items.push(InitListItem::Expr(self.parse_assignment_expr()));
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseBrace);

        InitList {
            items,
            span: self.span_from(start),
        }
    }

    // ── Type expressions ───────────────────────────────────────────────

    fn parse_type_expr(&mut self) -> TypeExpr {
        let start = self.current_span();
        let is_const = self.eat(TokenKind::Const);

        let kind = self.parse_type_kind();

        // Array dimensions: []
        let mut array_dimensions = 0u32;
        while self.at(TokenKind::OpenBracket) && self.peek_ahead(1) == TokenKind::CloseBracket {
            self.advance(); // [
            self.advance(); // ]
            array_dimensions += 1;
        }

        // Handle: @
        let mut is_handle = false;
        let mut is_handle_to_const = false;
        if self.eat(TokenKind::At) {
            is_handle = true;
            if self.eat(TokenKind::Const) {
                is_handle_to_const = true;
            }
        }

        // Reference: &
        let mut is_ref = false;
        let mut ref_modifier = None;
        if self.eat(TokenKind::Amp) {
            is_ref = true;
            if self.eat(TokenKind::In) {
                ref_modifier = Some(RefModifier::In);
            } else if self.eat(TokenKind::Out) {
                ref_modifier = Some(RefModifier::Out);
            } else if self.eat(TokenKind::InOut) {
                ref_modifier = Some(RefModifier::InOut);
            }
        }

        TypeExpr {
            kind,
            is_const,
            is_handle,
            is_handle_to_const,
            is_ref,
            ref_modifier,
            array_dimensions,
            span: self.span_from(start),
        }
    }

    fn parse_type_kind(&mut self) -> TypeExprKind {
        match self.peek_kind() {
            TokenKind::Void => {
                self.advance();
                TypeExprKind::Void
            }
            TokenKind::Auto => {
                self.advance();
                TypeExprKind::Auto
            }
            TokenKind::Question => {
                self.advance();
                TypeExprKind::Infer
            }
            TokenKind::Bool => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Bool)
            }
            TokenKind::Int8 => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Int8)
            }
            TokenKind::Int16 => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Int16)
            }
            TokenKind::Int => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Int)
            }
            TokenKind::Int64 => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Int64)
            }
            TokenKind::UInt8 => {
                self.advance();
                TypeExprKind::Primitive(PrimType::UInt8)
            }
            TokenKind::UInt16 => {
                self.advance();
                TypeExprKind::Primitive(PrimType::UInt16)
            }
            TokenKind::UInt => {
                self.advance();
                TypeExprKind::Primitive(PrimType::UInt)
            }
            TokenKind::UInt64 => {
                self.advance();
                TypeExprKind::Primitive(PrimType::UInt64)
            }
            TokenKind::Float => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Float)
            }
            TokenKind::Double => {
                self.advance();
                TypeExprKind::Primitive(PrimType::Double)
            }
            _ => {
                // Named or template type with optional scope
                let id = self.parse_scoped_identifier();

                // Template args: <Type, Type, ...>
                if self.at(TokenKind::Less) {
                    if let Some(args) = self.try_parse_template_args() {
                        return TypeExprKind::Template { base: id, args };
                    }
                }

                TypeExprKind::Named(id)
            }
        }
    }

    fn try_parse_template_args(&mut self) -> Option<Vec<TypeExpr>> {
        let saved = self.save_pos();
        self.advance(); // <

        let mut args = Vec::new();
        loop {
            if self.at(TokenKind::Greater) || self.at_eof() {
                break;
            }
            // Try to parse a type — if we fail, bail out and restore
            let ty = self.parse_type_expr();
            args.push(ty);

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        if self.eat(TokenKind::Greater) {
            Some(args)
        } else {
            self.restore_pos(saved);
            None
        }
    }

    fn can_start_type(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Const => true,
            TokenKind::Void
            | TokenKind::Auto
            | TokenKind::Question
            | TokenKind::Bool
            | TokenKind::Int8
            | TokenKind::Int16
            | TokenKind::Int
            | TokenKind::Int64
            | TokenKind::UInt8
            | TokenKind::UInt16
            | TokenKind::UInt
            | TokenKind::UInt64
            | TokenKind::Float
            | TokenKind::Double => true,
            TokenKind::Identifier => true,
            TokenKind::Scope => true, // ::Type
            _ => false,
        }
    }

    // ── Scoped identifiers ─────────────────────────────────────────────

    fn parse_scoped_identifier(&mut self) -> ScopedIdentifier {
        let start = self.current_span();
        let is_global = self.eat(TokenKind::Scope);

        let mut parts = Vec::new();
        let tok = self.expect(TokenKind::Identifier);
        parts.push(self.text(&tok).to_string());

        while self.at(TokenKind::Scope) && self.peek_ahead(1) == TokenKind::Identifier {
            self.advance(); // ::
            let tok = self.advance();
            parts.push(self.text(&tok).to_string());
        }

        let name = parts.pop().unwrap();

        ScopedIdentifier {
            is_global,
            scopes: parts,
            name,
            span: self.span_from(start),
        }
    }

    // ── Disambiguation predicates ──────────────────────────────────────

    /// Is the current position a function declaration?
    /// Heuristic: TYPE NAME '(' or just NAME '(' for constructors, or ~NAME for destructors.
    fn is_func_decl(&self) -> bool {
        self.is_func_decl_inner()
    }

    fn is_func_decl_inner(&self) -> bool {
        let mut pos = self.pos;

        // Skip destructor tilde
        if self.token_at(pos) == TokenKind::Tilde {
            return true;
        }

        // Skip const
        if self.token_at(pos) == TokenKind::Const {
            pos += 1;
        }

        // Skip type
        if self.token_at(pos).is_type_keyword()
            || self.token_at(pos) == TokenKind::Identifier
            || self.token_at(pos) == TokenKind::Scope
        {
            // Walk past the type (simple heuristic)
            pos = self.skip_type_at(pos);

            // Check for NAME (
            if self.token_at(pos) == TokenKind::Identifier
                && self.token_at(pos + 1) == TokenKind::OpenParen
            {
                return true;
            }
        }

        // Constructor: NAME (
        let pos = self.pos;
        if self.token_at(pos) == TokenKind::Identifier
            && self.token_at(pos + 1) == TokenKind::OpenParen
        {
            // Could be constructor or function call expression.
            // For now: treat as constructor if we're in a class context,
            // but at top level, need at least a simple identifier followed by '('.
            // Actually, at top level this would be ambiguous with expression statement.
            // We assume function decl if: after the param list there's a '{' or ';' or func attrs
            return self.check_after_params(pos + 1);
        }

        false
    }

    fn check_after_params(&self, paren_pos: usize) -> bool {
        // Find matching close paren
        let mut pos = paren_pos + 1; // skip (
        let mut depth = 1;
        while depth > 0 && self.token_at(pos) != TokenKind::Eof {
            match self.token_at(pos) {
                TokenKind::OpenParen => depth += 1,
                TokenKind::CloseParen => depth -= 1,
                _ => {}
            }
            pos += 1;
        }

        // After ), check for const
        if self.token_at(pos) == TokenKind::Const {
            pos += 1;
        }

        // Check for func attrs
        loop {
            if let Some(text) = self.identifier_text_at(pos) {
                match text {
                    "override" | "final" | "explicit" | "property" | "delete" => {
                        pos += 1;
                        continue;
                    }
                    _ => {}
                }
            }
            break;
        }

        // Should be followed by { or ;
        matches!(
            self.token_at(pos),
            TokenKind::OpenBrace | TokenKind::Semicolon
        )
    }

    fn is_var_decl(&self) -> bool {
        let mut pos = self.pos;

        // Skip const
        if self.token_at(pos) == TokenKind::Const {
            pos += 1;
        }

        // Must start with a type
        if !self.token_at(pos).is_type_keyword()
            && self.token_at(pos) != TokenKind::Identifier
            && self.token_at(pos) != TokenKind::Scope
        {
            return false;
        }

        // Walk past the type
        pos = self.skip_type_at(pos);

        // Should be followed by an identifier (variable name)
        if self.token_at(pos) != TokenKind::Identifier {
            return false;
        }
        pos += 1;

        // Then one of: = , ; (
        matches!(
            self.token_at(pos),
            TokenKind::Assign | TokenKind::Comma | TokenKind::Semicolon | TokenKind::OpenParen
        )
    }

    fn is_virtual_property_decl(&self) -> bool {
        let mut pos = self.pos;

        // Skip const
        if self.token_at(pos) == TokenKind::Const {
            pos += 1;
        }

        // Must start with a type
        if !self.token_at(pos).is_type_keyword()
            && self.token_at(pos) != TokenKind::Identifier
            && self.token_at(pos) != TokenKind::Scope
        {
            return false;
        }

        pos = self.skip_type_at(pos);

        // identifier
        if self.token_at(pos) != TokenKind::Identifier {
            return false;
        }
        pos += 1;

        // {
        self.token_at(pos) == TokenKind::OpenBrace
    }

    /// Skip past a type expression at the given position (for lookahead).
    fn skip_type_at(&self, mut pos: usize) -> usize {
        // scope: ::
        if self.token_at(pos) == TokenKind::Scope {
            pos += 1;
        }

        // Primitive type or identifier
        if self.token_at(pos).is_type_keyword() || self.token_at(pos) == TokenKind::Identifier {
            pos += 1;
        }

        // scope parts: ::name
        while self.token_at(pos) == TokenKind::Scope
            && self.token_at(pos + 1) == TokenKind::Identifier
        {
            pos += 2;
        }

        // Template args: < ... >
        if self.token_at(pos) == TokenKind::Less {
            let mut depth = 1;
            pos += 1;
            while depth > 0 && self.token_at(pos) != TokenKind::Eof {
                match self.token_at(pos) {
                    TokenKind::Less => depth += 1,
                    TokenKind::Greater => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
        }

        // Array: []
        while self.token_at(pos) == TokenKind::OpenBracket
            && self.token_at(pos + 1) == TokenKind::CloseBracket
        {
            pos += 2;
        }

        // Handle: @
        if self.token_at(pos) == TokenKind::At {
            pos += 1;
            // const after @
            if self.token_at(pos) == TokenKind::Const {
                pos += 1;
            }
        }

        // Reference: &
        if self.token_at(pos) == TokenKind::Amp {
            pos += 1;
            // in/out/inout
            match self.token_at(pos) {
                TokenKind::In | TokenKind::Out | TokenKind::InOut => {
                    pos += 1;
                }
                _ => {}
            }
        }

        pos
    }

    fn token_at(&self, pos: usize) -> TokenKind {
        self.tokens
            .get(pos)
            .map(|t| t.kind)
            .unwrap_or(TokenKind::Eof)
    }

    fn identifier_text_at(&self, pos: usize) -> Option<&str> {
        self.tokens.get(pos).and_then(|t| {
            if t.kind == TokenKind::Identifier {
                Some(t.text(self.source))
            } else {
                None
            }
        })
    }

    // ── Statements ─────────────────────────────────────────────────────

    fn parse_statement_block(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::OpenBrace);
        let mut stmts = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            stmts.push(self.parse_statement());
        }
        self.expect(TokenKind::CloseBrace);
        Stmt {
            kind: StmtKind::Block(stmts),
            span: self.span_from(start),
        }
    }

    fn parse_statement(&mut self) -> Stmt {
        match self.peek_kind() {
            TokenKind::OpenBrace => self.parse_statement_block(),
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::While => self.parse_while(),
            TokenKind::Do => self.parse_do_while(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => self.parse_break(),
            TokenKind::Continue => self.parse_continue(),
            TokenKind::Try => self.parse_try_catch(),
            TokenKind::Semicolon => {
                let start = self.current_span();
                self.advance();
                Stmt {
                    kind: StmtKind::Empty,
                    span: start,
                }
            }
            _ => {
                // Variable declaration or expression statement
                if self.is_var_decl() {
                    let start = self.current_span();
                    let decl = self.parse_var_decl(None);
                    Stmt {
                        kind: StmtKind::VarDecl(decl),
                        span: self.span_from(start),
                    }
                } else {
                    self.parse_expr_statement()
                }
            }
        }
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::If);
        self.expect(TokenKind::OpenParen);
        let condition = self.parse_assignment_expr();
        self.expect(TokenKind::CloseParen);
        let then_body = Box::new(self.parse_statement());
        let else_body = if self.eat(TokenKind::Else) {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        Stmt {
            kind: StmtKind::If {
                condition,
                then_body,
                else_body,
            },
            span: self.span_from(start),
        }
    }

    fn parse_for(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::For);
        self.expect(TokenKind::OpenParen);

        // Init: var decl or expression statement
        let init = if self.at(TokenKind::Semicolon) {
            self.advance();
            None
        } else if self.is_var_decl() {
            Some(Box::new(Stmt {
                kind: StmtKind::VarDecl(self.parse_var_decl(None)),
                span: self.span_from(start),
            }))
        } else {
            let expr = self.parse_assignment_expr();
            let expr_span = expr.span;
            self.expect(TokenKind::Semicolon);
            Some(Box::new(Stmt {
                kind: StmtKind::ExprStatement(expr),
                span: expr_span,
            }))
        };

        // Condition
        let condition = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_assignment_expr())
        };
        self.expect(TokenKind::Semicolon);

        // Increment (comma-separated assignments)
        let mut increment = Vec::new();
        while !self.at(TokenKind::CloseParen) && !self.at_eof() {
            increment.push(self.parse_assignment_expr());
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseParen);

        let body = Box::new(self.parse_statement());

        Stmt {
            kind: StmtKind::For {
                init,
                condition,
                increment,
                body,
            },
            span: self.span_from(start),
        }
    }

    fn parse_while(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::While);
        self.expect(TokenKind::OpenParen);
        let condition = self.parse_assignment_expr();
        self.expect(TokenKind::CloseParen);
        let body = Box::new(self.parse_statement());
        Stmt {
            kind: StmtKind::While { condition, body },
            span: self.span_from(start),
        }
    }

    fn parse_do_while(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::Do);
        let body = Box::new(self.parse_statement());
        self.expect(TokenKind::While);
        self.expect(TokenKind::OpenParen);
        let condition = self.parse_assignment_expr();
        self.expect(TokenKind::CloseParen);
        self.expect(TokenKind::Semicolon);
        Stmt {
            kind: StmtKind::DoWhile { body, condition },
            span: self.span_from(start),
        }
    }

    fn parse_switch(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::Switch);
        self.expect(TokenKind::OpenParen);
        let expr = self.parse_assignment_expr();
        self.expect(TokenKind::CloseParen);
        self.expect(TokenKind::OpenBrace);

        let mut cases = Vec::new();
        while !self.at(TokenKind::CloseBrace) && !self.at_eof() {
            cases.push(self.parse_case());
        }
        self.expect(TokenKind::CloseBrace);

        Stmt {
            kind: StmtKind::Switch { expr, cases },
            span: self.span_from(start),
        }
    }

    fn parse_case(&mut self) -> CaseClause {
        let start = self.current_span();
        let label = if self.eat(TokenKind::Case) {
            CaseLabel::Expr(self.parse_expr())
        } else {
            self.expect(TokenKind::Default);
            CaseLabel::Default
        };
        self.expect(TokenKind::Colon);

        let mut body = Vec::new();
        while !self.at(TokenKind::Case)
            && !self.at(TokenKind::Default)
            && !self.at(TokenKind::CloseBrace)
            && !self.at_eof()
        {
            body.push(self.parse_statement());
        }

        CaseClause {
            label,
            body,
            span: self.span_from(start),
        }
    }

    fn parse_return(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::Return);
        let value = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_assignment_expr())
        };
        self.expect(TokenKind::Semicolon);
        Stmt {
            kind: StmtKind::Return(value),
            span: self.span_from(start),
        }
    }

    fn parse_break(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::Break);
        self.expect(TokenKind::Semicolon);
        Stmt {
            kind: StmtKind::Break,
            span: self.span_from(start),
        }
    }

    fn parse_continue(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::Continue);
        self.expect(TokenKind::Semicolon);
        Stmt {
            kind: StmtKind::Continue,
            span: self.span_from(start),
        }
    }

    fn parse_try_catch(&mut self) -> Stmt {
        let start = self.current_span();
        self.expect(TokenKind::Try);
        let try_body = Box::new(self.parse_statement_block());
        self.expect(TokenKind::Catch);
        let catch_body = Box::new(self.parse_statement_block());
        Stmt {
            kind: StmtKind::TryCatch {
                try_body,
                catch_body,
            },
            span: self.span_from(start),
        }
    }

    fn parse_expr_statement(&mut self) -> Stmt {
        let start = self.current_span();
        let expr = self.parse_assignment_expr();
        self.expect(TokenKind::Semicolon);
        Stmt {
            kind: StmtKind::ExprStatement(expr),
            span: self.span_from(start),
        }
    }

    // ── Expressions ────────────────────────────────────────────────────

    /// Top-level expression: assignment.
    pub fn parse_expr(&mut self) -> Expr {
        self.parse_assignment_expr()
    }

    /// Assignment is right-associative: `a = b = c` parses as `a = (b = c)`.
    fn parse_assignment_expr(&mut self) -> Expr {
        let start = self.current_span();
        let lhs = self.parse_ternary_expr();

        if let Some(op) = self.try_assignment_op() {
            let rhs = self.parse_assignment_expr(); // right-recursive
            Expr {
                kind: ExprKind::Assign {
                    target: Box::new(lhs),
                    op,
                    value: Box::new(rhs),
                },
                span: self.span_from(start),
            }
        } else {
            lhs
        }
    }

    fn try_assignment_op(&mut self) -> Option<AssignOp> {
        let op = match self.peek_kind() {
            TokenKind::Assign => AssignOp::Assign,
            TokenKind::AddAssign => AssignOp::AddAssign,
            TokenKind::SubAssign => AssignOp::SubAssign,
            TokenKind::MulAssign => AssignOp::MulAssign,
            TokenKind::DivAssign => AssignOp::DivAssign,
            TokenKind::ModAssign => AssignOp::ModAssign,
            TokenKind::PowAssign => AssignOp::PowAssign,
            TokenKind::OrAssign => AssignOp::OrAssign,
            TokenKind::AndAssign => AssignOp::AndAssign,
            TokenKind::XorAssign => AssignOp::XorAssign,
            TokenKind::ShiftLeftAssign => AssignOp::ShiftLeftAssign,
            TokenKind::ShiftRightLAssign => AssignOp::ShiftRightLAssign,
            TokenKind::ShiftRightAAssign => AssignOp::ShiftRightAAssign,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    /// Ternary conditional: `cond ? then : else`.
    fn parse_ternary_expr(&mut self) -> Expr {
        let start = self.current_span();
        let condition = self.parse_binary_expr(0);

        if self.eat(TokenKind::Question) {
            let then_expr = self.parse_assignment_expr();
            self.expect(TokenKind::Colon);
            let else_expr = self.parse_assignment_expr();
            Expr {
                kind: ExprKind::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                span: self.span_from(start),
            }
        } else {
            condition
        }
    }

    /// Pratt parser for binary expressions.
    fn parse_binary_expr(&mut self, min_bp: u8) -> Expr {
        let start = self.current_span();
        let mut lhs = self.parse_unary_expr();

        loop {
            let op = match self.try_binary_op() {
                Some(op) => op,
                None => break,
            };

            let (l_bp, r_bp) = op.binding_power();
            if l_bp < min_bp {
                break;
            }

            self.advance(); // consume the operator token
            let rhs = self.parse_binary_expr(r_bp);

            lhs = Expr {
                kind: ExprKind::BinaryOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span: self.span_from(start),
            };
        }

        lhs
    }

    fn try_binary_op(&self) -> Option<BinOp> {
        match self.peek_kind() {
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Percent => Some(BinOp::Mod),
            TokenKind::StarStar => Some(BinOp::Pow),
            TokenKind::Equal => Some(BinOp::Eq),
            TokenKind::NotEqual => Some(BinOp::NotEq),
            TokenKind::Less => Some(BinOp::Less),
            TokenKind::LessEqual => Some(BinOp::LessEq),
            TokenKind::Greater => Some(BinOp::Greater),
            TokenKind::GreaterEqual => Some(BinOp::GreaterEq),
            TokenKind::Is => Some(BinOp::Is),
            TokenKind::NotIs => Some(BinOp::NotIs),
            TokenKind::And => Some(BinOp::LogicAnd),
            TokenKind::Or => Some(BinOp::LogicOr),
            TokenKind::Xor => Some(BinOp::LogicXor),
            TokenKind::Amp => Some(BinOp::BitAnd),
            TokenKind::Pipe => Some(BinOp::BitOr),
            TokenKind::Caret => Some(BinOp::BitXor),
            TokenKind::ShiftLeft => Some(BinOp::ShiftLeft),
            TokenKind::ShiftRight => Some(BinOp::ShiftRight),
            TokenKind::ShiftRightArith => Some(BinOp::ShiftRightArith),
            _ => None,
        }
    }

    fn parse_unary_expr(&mut self) -> Expr {
        let start = self.current_span();

        match self.peek_kind() {
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::Neg,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            TokenKind::Plus => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::Pos,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::LogicNot,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::BitNot,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            TokenKind::Inc => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::PreInc,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            TokenKind::Dec => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::PreDec,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            TokenKind::At => {
                self.advance();
                let operand = self.parse_unary_expr();
                Expr {
                    kind: ExprKind::UnaryOp {
                        op: UnaryOp::HandleOf,
                        operand: Box::new(operand),
                    },
                    span: self.span_from(start),
                }
            }
            _ => self.parse_postfix_expr(),
        }
    }

    fn parse_postfix_expr(&mut self) -> Expr {
        let start = self.current_span();
        let mut expr = self.parse_primary_expr();

        loop {
            match self.peek_kind() {
                TokenKind::Inc => {
                    self.advance();
                    expr = Expr {
                        kind: ExprKind::PostfixOp {
                            op: PostfixOp::Inc,
                            operand: Box::new(expr),
                        },
                        span: self.span_from(start),
                    };
                }
                TokenKind::Dec => {
                    self.advance();
                    expr = Expr {
                        kind: ExprKind::PostfixOp {
                            op: PostfixOp::Dec,
                            operand: Box::new(expr),
                        },
                        span: self.span_from(start),
                    };
                }
                TokenKind::Dot => {
                    self.advance();
                    // member.method(args) or member.field
                    let name_tok = self.expect(TokenKind::Identifier);
                    let member = self.text(&name_tok).to_string();

                    expr = Expr {
                        kind: ExprKind::MemberAccess {
                            object: Box::new(expr),
                            member,
                        },
                        span: self.span_from(start),
                    };
                }
                TokenKind::OpenBracket => {
                    self.advance();
                    let index = self.parse_assignment_expr();
                    self.expect(TokenKind::CloseBracket);
                    expr = Expr {
                        kind: ExprKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        span: self.span_from(start),
                    };
                }
                TokenKind::OpenParen => {
                    let args = self.parse_func_args();
                    expr = Expr {
                        kind: ExprKind::FunctionCall {
                            callee: Box::new(expr),
                            args,
                        },
                        span: self.span_from(start),
                    };
                }
                _ => break,
            }
        }

        expr
    }

    fn parse_func_args(&mut self) -> Vec<FuncArg> {
        self.expect(TokenKind::OpenParen);
        let mut args = Vec::new();

        while !self.at(TokenKind::CloseParen) && !self.at_eof() {
            let arg_start = self.current_span();

            // Named argument: `name: value`
            let name = if self.at(TokenKind::Identifier) && self.peek_ahead(1) == TokenKind::Colon {
                let tok = self.advance();
                let n = self.text(&tok).to_string();
                self.advance(); // :
                Some(n)
            } else {
                None
            };

            let expr = self.parse_assignment_expr();
            args.push(FuncArg {
                name,
                expr,
                span: self.span_from(arg_start),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseParen);
        args
    }

    fn parse_primary_expr(&mut self) -> Expr {
        let start = self.current_span();

        match self.peek_kind() {
            // Literals
            TokenKind::IntConstant | TokenKind::BitsConstant => {
                let tok = self.advance();
                let text = self.text(&tok);
                let value = self.parse_int_value(text);
                Expr {
                    kind: ExprKind::IntLiteral(value),
                    span: self.span_from(start),
                }
            }
            TokenKind::FloatConstant => {
                let tok = self.advance();
                let text = self.text(&tok);
                let value = self.parse_float_value(text);
                Expr {
                    kind: ExprKind::FloatLiteral(value),
                    span: self.span_from(start),
                }
            }
            TokenKind::DoubleConstant => {
                let tok = self.advance();
                let text = self.text(&tok);
                let value = self.parse_double_value(text);
                Expr {
                    kind: ExprKind::DoubleLiteral(value),
                    span: self.span_from(start),
                }
            }
            TokenKind::StringConstant
            | TokenKind::MultilineStringConstant
            | TokenKind::HeredocStringConstant => {
                let tok = self.advance();
                let value = self.parse_string_content(&tok);
                // String concatenation: adjacent string literals
                let mut combined = value;
                while matches!(
                    self.peek_kind(),
                    TokenKind::StringConstant
                        | TokenKind::MultilineStringConstant
                        | TokenKind::HeredocStringConstant
                ) {
                    let tok = self.advance();
                    combined.push_str(&self.parse_string_content(&tok));
                }
                Expr {
                    kind: ExprKind::StringLiteral(combined),
                    span: self.span_from(start),
                }
            }
            TokenKind::True => {
                self.advance();
                Expr {
                    kind: ExprKind::BoolLiteral(true),
                    span: self.span_from(start),
                }
            }
            TokenKind::False => {
                self.advance();
                Expr {
                    kind: ExprKind::BoolLiteral(false),
                    span: self.span_from(start),
                }
            }
            TokenKind::Null => {
                self.advance();
                Expr {
                    kind: ExprKind::Null,
                    span: self.span_from(start),
                }
            }
            TokenKind::Void => {
                self.advance();
                Expr {
                    kind: ExprKind::Void,
                    span: self.span_from(start),
                }
            }

            // Parenthesized expression
            TokenKind::OpenParen => {
                self.advance();
                let expr = self.parse_assignment_expr();
                self.expect(TokenKind::CloseParen);
                expr
            }

            // Init list in expression
            TokenKind::OpenBrace => {
                let list = self.parse_init_list();
                Expr {
                    kind: ExprKind::InitList(list),
                    span: self.span_from(start),
                }
            }

            // Cast: cast<Type>(expr)
            TokenKind::Cast => {
                self.advance();
                self.expect(TokenKind::Less);
                let type_expr = self.parse_type_expr();
                self.expect(TokenKind::Greater);
                self.expect(TokenKind::OpenParen);
                let expr = self.parse_assignment_expr();
                self.expect(TokenKind::CloseParen);
                Expr {
                    kind: ExprKind::Cast {
                        type_expr,
                        expr: Box::new(expr),
                    },
                    span: self.span_from(start),
                }
            }

            // Lambda or identifier "function"
            TokenKind::Identifier if self.identifier_is("function") => {
                // Lambda: function(params) { body }
                self.advance(); // 'function'
                let params = self.parse_param_list();
                let body = Box::new(self.parse_statement_block());
                Expr {
                    kind: ExprKind::Lambda { params, body },
                    span: self.span_from(start),
                }
            }

            // Identifier or constructor call
            TokenKind::Identifier | TokenKind::Scope => {
                // Could be: variable access, function call, or constructor call
                // Constructor: primitive(args) or ScopedType(args)
                if self.is_construct_call() {
                    let type_expr = self.parse_type_expr();
                    let args = self.parse_func_args();
                    Expr {
                        kind: ExprKind::ConstructCall { type_expr, args },
                        span: self.span_from(start),
                    }
                } else {
                    let id = self.parse_scoped_identifier();
                    Expr {
                        kind: ExprKind::Identifier(id),
                        span: self.span_from(start),
                    }
                }
            }

            // Type keywords as constructor calls: int(5), float(3.0)
            kind if kind.is_type_keyword()
                && kind != TokenKind::Void
                && kind != TokenKind::Auto =>
            {
                let type_expr = self.parse_type_expr();
                if self.at(TokenKind::OpenParen) {
                    let args = self.parse_func_args();
                    Expr {
                        kind: ExprKind::ConstructCall { type_expr, args },
                        span: self.span_from(start),
                    }
                } else {
                    // Type used as expression? This is unusual, emit error
                    self.error("unexpected type keyword in expression".to_string());
                    Expr {
                        kind: ExprKind::Void,
                        span: self.span_from(start),
                    }
                }
            }

            _ => {
                let tok = self.advance();
                self.error(format!("unexpected token '{}' in expression", tok.kind));
                Expr {
                    kind: ExprKind::Void,
                    span: self.span_from(start),
                }
            }
        }
    }

    /// Check if the current position is a constructor call: Type(args).
    /// This is tricky — we need to distinguish `Type(args)` from `func(args)`.
    fn is_construct_call(&self) -> bool {
        let mut pos = self.pos;

        // Skip scope
        if self.token_at(pos) == TokenKind::Scope {
            pos += 1;
        }

        if self.token_at(pos) != TokenKind::Identifier {
            return false;
        }

        // Walk scope parts
        while self.token_at(pos) == TokenKind::Identifier
            && self.token_at(pos + 1) == TokenKind::Scope
        {
            pos += 2;
        }

        if self.token_at(pos) != TokenKind::Identifier {
            return false;
        }
        pos += 1;

        // Template args
        if self.token_at(pos) == TokenKind::Less {
            let mut depth = 1;
            pos += 1;
            while depth > 0 && self.token_at(pos) != TokenKind::Eof {
                match self.token_at(pos) {
                    TokenKind::Less => depth += 1,
                    TokenKind::Greater => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
        }

        // Array dimension: []
        while self.token_at(pos) == TokenKind::OpenBracket
            && self.token_at(pos + 1) == TokenKind::CloseBracket
        {
            pos += 2;
        }

        // Must have ( immediately — but also need scope prefix to disambiguate
        // from function call. A constructor needs at least a scope prefix or template args
        // to be unambiguous. For now: treat as constructor if there's a scope or template.
        if self.token_at(pos) == TokenKind::OpenParen {
            let orig_pos = self.pos;
            // Has scope prefix?
            if self.token_at(orig_pos) == TokenKind::Scope {
                return true;
            }
            // Has more than just identifier?
            if self.token_at(orig_pos) == TokenKind::Identifier {
                let next = self.token_at(orig_pos + 1);
                // Scope prefix: NS::Type(
                if next == TokenKind::Scope {
                    return true;
                }
                // Template: Type<T>(
                if next == TokenKind::Less && pos > orig_pos + 1 {
                    return true;
                }
                // Array: Type[](
                if next == TokenKind::OpenBracket {
                    return true;
                }
            }
        }

        false
    }

    // ── Literal parsing helpers ────────────────────────────────────────

    fn parse_int_value(&self, text: &str) -> i64 {
        if text.starts_with("0x") || text.starts_with("0X") {
            i64::from_str_radix(&text[2..], 16).unwrap_or(0)
        } else if text.starts_with("0o") || text.starts_with("0O") {
            i64::from_str_radix(&text[2..], 8).unwrap_or(0)
        } else if text.starts_with("0b") || text.starts_with("0B") {
            i64::from_str_radix(&text[2..], 2).unwrap_or(0)
        } else if text.starts_with("0d") || text.starts_with("0D") {
            text[2..].parse().unwrap_or(0)
        } else {
            text.parse().unwrap_or(0)
        }
    }

    fn parse_float_value(&self, text: &str) -> f64 {
        let text = text.trim_end_matches('f').trim_end_matches('F');
        text.parse().unwrap_or(0.0)
    }

    fn parse_double_value(&self, text: &str) -> f64 {
        let text = text.trim_end_matches('d').trim_end_matches('D');
        text.parse().unwrap_or(0.0)
    }

    fn parse_string_content(&self, token: &Token) -> String {
        let text = self.text(token);
        match token.kind {
            TokenKind::StringConstant => {
                // Strip quotes and process escape sequences
                let inner = &text[1..text.len() - 1];
                self.unescape_string(inner)
            }
            TokenKind::MultilineStringConstant => {
                let inner = &text[1..text.len() - 1];
                self.unescape_string(inner)
            }
            TokenKind::HeredocStringConstant => {
                // Heredoc: strip """ delimiters, no escaping
                text[3..text.len() - 3].to_string()
            }
            _ => text.to_string(),
        }
    }

    fn unescape_string(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some('0') => result.push('\0'),
                    Some('x') => {
                        let hex: String = chars.by_ref().take(2).collect();
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                result.push(ch);
                            }
                        }
                    }
                    Some('u') => {
                        let hex: String = chars.by_ref().take(4).collect();
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                result.push(ch);
                            }
                        }
                    }
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Script {
        let mut parser = Parser::new(source);
        parser.parse_script()
    }

    fn parse_check(source: &str) -> (Script, bool) {
        let mut parser = Parser::new(source);
        let script = parser.parse_script();
        let has_errors = parser.has_errors();
        (script, has_errors)
    }

    fn parse_one_decl(source: &str) -> Declaration {
        let script = parse(source);
        assert_eq!(script.declarations.len(), 1, "expected 1 declaration");
        script.declarations.into_iter().next().unwrap()
    }

    fn parse_one_stmt(source: &str) -> Stmt {
        // Wrap in a function to parse as statement
        let wrapped = format!("void test() {{ {} }}", source);
        let script = parse(&wrapped);
        let decl = script.declarations.into_iter().next().unwrap();
        if let Declaration::Function(f) = decl {
            if let Some(body) = f.body {
                if let StmtKind::Block(mut stmts) = body.kind {
                    return stmts.remove(0);
                }
            }
        }
        panic!("failed to extract statement");
    }

    fn parse_one_expr(source: &str) -> Expr {
        let wrapped = format!("void test() {{ {}; }}", source);
        let script = parse(&wrapped);
        let decl = script.declarations.into_iter().next().unwrap();
        if let Declaration::Function(f) = decl {
            if let Some(body) = f.body {
                if let StmtKind::Block(mut stmts) = body.kind {
                    if let StmtKind::ExprStatement(expr) = stmts.remove(0).kind {
                        return expr;
                    }
                }
            }
        }
        panic!("failed to extract expression");
    }

    // ── Declaration tests ──────────────────────────────────────────────

    #[test]
    fn empty_script() {
        let script = parse("");
        assert!(script.declarations.is_empty());
    }

    #[test]
    fn simple_function() {
        let decl = parse_one_decl("int add(int a, int b) { return a + b; }");
        if let Declaration::Function(f) = decl {
            assert_eq!(f.name, "add");
            assert_eq!(f.params.len(), 2);
            assert!(f.return_type.is_some());
            assert!(f.body.is_some());
        } else {
            panic!("expected function declaration");
        }
    }

    #[test]
    fn void_function() {
        let decl = parse_one_decl("void doStuff() {}");
        if let Declaration::Function(f) = decl {
            assert_eq!(f.name, "doStuff");
            assert_eq!(f.params.len(), 0);
        } else {
            panic!("expected function declaration");
        }
    }

    #[test]
    fn void_param_list() {
        let decl = parse_one_decl("void foo(void) {}");
        if let Declaration::Function(f) = decl {
            assert_eq!(f.params.len(), 0);
        } else {
            panic!("expected function declaration");
        }
    }

    #[test]
    fn class_declaration() {
        let decl = parse_one_decl("class Foo { int x; void bar() {} }");
        if let Declaration::Class(c) = decl {
            assert_eq!(c.name, "Foo");
            assert_eq!(c.members.len(), 2);
        } else {
            panic!("expected class declaration");
        }
    }

    #[test]
    fn class_with_inheritance() {
        let decl = parse_one_decl("class Derived : Base, IInterface {}");
        if let Declaration::Class(c) = decl {
            assert_eq!(c.name, "Derived");
            assert_eq!(c.base_classes.len(), 2);
            assert_eq!(c.base_classes[0].name, "Base");
            assert_eq!(c.base_classes[1].name, "IInterface");
        } else {
            panic!("expected class declaration");
        }
    }

    #[test]
    fn class_forward_declaration() {
        let decl = parse_one_decl("class Foo;");
        if let Declaration::Class(c) = decl {
            assert_eq!(c.name, "Foo");
            assert!(c.members.is_empty());
        } else {
            panic!("expected class declaration");
        }
    }

    #[test]
    fn interface_declaration() {
        let decl = parse_one_decl("interface IFoo { void bar(); }");
        if let Declaration::Interface(i) = decl {
            assert_eq!(i.name, "IFoo");
            assert_eq!(i.methods.len(), 1);
            assert_eq!(i.methods[0].name, "bar");
        } else {
            panic!("expected interface declaration");
        }
    }

    #[test]
    fn enum_declaration() {
        let decl = parse_one_decl("enum Color { Red, Green, Blue = 5 }");
        if let Declaration::Enum(e) = decl {
            assert_eq!(e.name, "Color");
            assert_eq!(e.values.len(), 3);
            assert_eq!(e.values[0].name, "Red");
            assert!(e.values[0].value.is_none());
            assert_eq!(e.values[2].name, "Blue");
            assert!(e.values[2].value.is_some());
        } else {
            panic!("expected enum declaration");
        }
    }

    #[test]
    fn typedef_declaration() {
        let decl = parse_one_decl("typedef float real;");
        if let Declaration::Typedef(t) = decl {
            assert_eq!(t.name, "real");
        } else {
            panic!("expected typedef declaration");
        }
    }

    #[test]
    fn funcdef_declaration() {
        let decl = parse_one_decl("funcdef void Callback(int);");
        if let Declaration::Funcdef(f) = decl {
            assert_eq!(f.name, "Callback");
            assert_eq!(f.params.len(), 1);
        } else {
            panic!("expected funcdef declaration");
        }
    }

    #[test]
    fn namespace_declaration() {
        let decl = parse_one_decl("namespace NS { void foo() {} }");
        if let Declaration::Namespace(n) = decl {
            assert_eq!(n.name, vec!["NS"]);
            assert_eq!(n.declarations.len(), 1);
        } else {
            panic!("expected namespace declaration");
        }
    }

    #[test]
    fn nested_namespace() {
        let decl = parse_one_decl("namespace A::B { void foo() {} }");
        if let Declaration::Namespace(n) = decl {
            assert_eq!(n.name, vec!["A", "B"]);
        } else {
            panic!("expected namespace declaration");
        }
    }

    #[test]
    fn global_variable() {
        let decl = parse_one_decl("int globalVar = 42;");
        if let Declaration::GlobalVar(v) = decl {
            assert_eq!(v.declarators[0].name, "globalVar");
        } else {
            panic!("expected global variable");
        }
    }

    #[test]
    fn shared_class() {
        let decl = parse_one_decl("shared class Foo {}");
        if let Declaration::Class(c) = decl {
            assert_eq!(c.name, "Foo");
            assert!(c.flags.contains(&ClassFlag::Shared));
        } else {
            panic!("expected class declaration");
        }
    }

    // ── Statement tests ────────────────────────────────────────────────

    #[test]
    fn if_statement() {
        let stmt = parse_one_stmt("if (x > 0) return 1;");
        assert!(matches!(stmt.kind, StmtKind::If { .. }));
    }

    #[test]
    fn if_else_statement() {
        let stmt = parse_one_stmt("if (x > 0) return 1; else return 0;");
        if let StmtKind::If { else_body, .. } = stmt.kind {
            assert!(else_body.is_some());
        } else {
            panic!("expected if statement");
        }
    }

    #[test]
    fn for_loop() {
        let stmt = parse_one_stmt("for (int i = 0; i < 10; i++) x += i;");
        if let StmtKind::For {
            init,
            condition,
            increment,
            ..
        } = stmt.kind
        {
            assert!(init.is_some());
            assert!(condition.is_some());
            assert_eq!(increment.len(), 1);
        } else {
            panic!("expected for loop");
        }
    }

    #[test]
    fn while_loop() {
        let stmt = parse_one_stmt("while (running) update();");
        assert!(matches!(stmt.kind, StmtKind::While { .. }));
    }

    #[test]
    fn do_while_loop() {
        let stmt = parse_one_stmt("do { x++; } while (x < 10);");
        assert!(matches!(stmt.kind, StmtKind::DoWhile { .. }));
    }

    #[test]
    fn switch_statement() {
        let stmt = parse_one_stmt("switch (x) { case 1: break; default: return; }");
        if let StmtKind::Switch { cases, .. } = stmt.kind {
            assert_eq!(cases.len(), 2);
        } else {
            panic!("expected switch");
        }
    }

    #[test]
    fn return_statement() {
        let stmt = parse_one_stmt("return 42;");
        if let StmtKind::Return(Some(expr)) = stmt.kind {
            assert!(matches!(expr.kind, ExprKind::IntLiteral(42)));
        } else {
            panic!("expected return");
        }
    }

    #[test]
    fn return_void() {
        let stmt = parse_one_stmt("return;");
        assert!(matches!(stmt.kind, StmtKind::Return(None)));
    }

    #[test]
    fn break_continue() {
        assert!(matches!(parse_one_stmt("break;").kind, StmtKind::Break));
        assert!(matches!(
            parse_one_stmt("continue;").kind,
            StmtKind::Continue
        ));
    }

    #[test]
    fn try_catch() {
        let stmt = parse_one_stmt("try { foo(); } catch { bar(); }");
        assert!(matches!(stmt.kind, StmtKind::TryCatch { .. }));
    }

    #[test]
    fn var_declaration() {
        let stmt = parse_one_stmt("int x = 5;");
        if let StmtKind::VarDecl(v) = stmt.kind {
            assert_eq!(v.declarators[0].name, "x");
        } else {
            panic!("expected var decl");
        }
    }

    #[test]
    fn multi_var_declaration() {
        let stmt = parse_one_stmt("int x = 1, y = 2;");
        if let StmtKind::VarDecl(v) = stmt.kind {
            assert_eq!(v.declarators.len(), 2);
            assert_eq!(v.declarators[0].name, "x");
            assert_eq!(v.declarators[1].name, "y");
        } else {
            panic!("expected var decl");
        }
    }

    // ── Expression tests ───────────────────────────────────────────────

    #[test]
    fn integer_literal() {
        let expr = parse_one_expr("42");
        assert!(matches!(expr.kind, ExprKind::IntLiteral(42)));
    }

    #[test]
    fn hex_literal() {
        let expr = parse_one_expr("0xFF");
        assert!(matches!(expr.kind, ExprKind::IntLiteral(255)));
    }

    #[test]
    fn float_literal() {
        let expr = parse_one_expr("3.14f");
        if let ExprKind::FloatLiteral(v) = expr.kind {
            assert!((v - 3.14).abs() < 0.001);
        } else {
            panic!("expected float literal");
        }
    }

    #[test]
    fn string_literal() {
        let expr = parse_one_expr("\"hello\"");
        assert!(matches!(expr.kind, ExprKind::StringLiteral(ref s) if s == "hello"));
    }

    #[test]
    fn bool_literals() {
        assert!(matches!(
            parse_one_expr("true").kind,
            ExprKind::BoolLiteral(true)
        ));
        assert!(matches!(
            parse_one_expr("false").kind,
            ExprKind::BoolLiteral(false)
        ));
    }

    #[test]
    fn null_literal() {
        assert!(matches!(parse_one_expr("null").kind, ExprKind::Null));
    }

    #[test]
    fn binary_addition() {
        let expr = parse_one_expr("a + b");
        if let ExprKind::BinaryOp { op, .. } = expr.kind {
            assert_eq!(op, BinOp::Add);
        } else {
            panic!("expected binary op");
        }
    }

    #[test]
    fn operator_precedence() {
        // a + b * c should parse as a + (b * c)
        let expr = parse_one_expr("a + b * c");
        if let ExprKind::BinaryOp { op, rhs, .. } = expr.kind {
            assert_eq!(op, BinOp::Add);
            assert!(matches!(
                rhs.kind,
                ExprKind::BinaryOp { op: BinOp::Mul, .. }
            ));
        } else {
            panic!("expected binary op");
        }
    }

    #[test]
    fn comparison_and_logic() {
        let expr = parse_one_expr("a > 0 && b < 10");
        if let ExprKind::BinaryOp { op, .. } = expr.kind {
            assert_eq!(op, BinOp::LogicAnd);
        } else {
            panic!("expected binary op");
        }
    }

    #[test]
    fn assignment() {
        let expr = parse_one_expr("x = 5");
        assert!(matches!(
            expr.kind,
            ExprKind::Assign {
                op: AssignOp::Assign,
                ..
            }
        ));
    }

    #[test]
    fn compound_assignment() {
        let expr = parse_one_expr("x += 5");
        assert!(matches!(
            expr.kind,
            ExprKind::Assign {
                op: AssignOp::AddAssign,
                ..
            }
        ));
    }

    #[test]
    fn unary_negation() {
        let expr = parse_one_expr("-x");
        assert!(matches!(
            expr.kind,
            ExprKind::UnaryOp {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn pre_increment() {
        let expr = parse_one_expr("++x");
        assert!(matches!(
            expr.kind,
            ExprKind::UnaryOp {
                op: UnaryOp::PreInc,
                ..
            }
        ));
    }

    #[test]
    fn post_increment() {
        let expr = parse_one_expr("x++");
        assert!(matches!(
            expr.kind,
            ExprKind::PostfixOp {
                op: PostfixOp::Inc,
                ..
            }
        ));
    }

    #[test]
    fn member_access() {
        let expr = parse_one_expr("obj.member");
        if let ExprKind::MemberAccess { member, .. } = expr.kind {
            assert_eq!(member, "member");
        } else {
            panic!("expected member access");
        }
    }

    #[test]
    fn function_call() {
        let expr = parse_one_expr("foo(1, 2)");
        if let ExprKind::FunctionCall { args, .. } = expr.kind {
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected function call");
        }
    }

    #[test]
    fn method_call_chain() {
        let expr = parse_one_expr("obj.method(1).field");
        // Should be MemberAccess(FunctionCall(MemberAccess(Identifier)))
        if let ExprKind::MemberAccess { member, object } = expr.kind {
            assert_eq!(member, "field");
            assert!(matches!(object.kind, ExprKind::FunctionCall { .. }));
        } else {
            panic!("expected member access");
        }
    }

    #[test]
    fn index_access() {
        let expr = parse_one_expr("arr[0]");
        assert!(matches!(expr.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn ternary_expr() {
        let expr = parse_one_expr("x > 0 ? 1 : 0");
        assert!(matches!(expr.kind, ExprKind::Ternary { .. }));
    }

    #[test]
    fn cast_expr() {
        let expr = parse_one_expr("cast<int>(x)");
        if let ExprKind::Cast { type_expr, .. } = expr.kind {
            assert!(matches!(
                type_expr.kind,
                TypeExprKind::Primitive(PrimType::Int)
            ));
        } else {
            panic!("expected cast");
        }
    }

    #[test]
    fn parenthesized_expr() {
        let expr = parse_one_expr("(a + b) * c");
        if let ExprKind::BinaryOp { op, lhs, .. } = expr.kind {
            assert_eq!(op, BinOp::Mul);
            assert!(matches!(
                lhs.kind,
                ExprKind::BinaryOp { op: BinOp::Add, .. }
            ));
        } else {
            panic!("expected binary op");
        }
    }

    #[test]
    fn power_right_associative() {
        // a ** b ** c should parse as a ** (b ** c)
        let expr = parse_one_expr("a ** b ** c");
        if let ExprKind::BinaryOp { op, rhs, .. } = expr.kind {
            assert_eq!(op, BinOp::Pow);
            assert!(matches!(
                rhs.kind,
                ExprKind::BinaryOp { op: BinOp::Pow, .. }
            ));
        } else {
            panic!("expected binary op");
        }
    }

    #[test]
    fn handle_type() {
        let decl = parse_one_decl("Foo@ handle;");
        if let Declaration::GlobalVar(v) = decl {
            assert!(v.type_expr.is_handle);
            assert_eq!(v.declarators[0].name, "handle");
        } else {
            panic!("expected global var");
        }
    }

    #[test]
    fn const_handle_type() {
        let decl = parse_one_decl("const Foo@ handle;");
        if let Declaration::GlobalVar(v) = decl {
            assert!(v.type_expr.is_const);
            assert!(v.type_expr.is_handle);
        } else {
            panic!("expected global var");
        }
    }

    #[test]
    fn array_type() {
        let decl = parse_one_decl("int[] arr;");
        if let Declaration::GlobalVar(v) = decl {
            assert_eq!(v.type_expr.array_dimensions, 1);
        } else {
            panic!("expected global var");
        }
    }

    #[test]
    fn ref_param() {
        let decl = parse_one_decl("void foo(int &in x) {}");
        if let Declaration::Function(f) = decl {
            assert!(f.params[0].type_expr.is_ref);
            assert_eq!(f.params[0].type_expr.ref_modifier, Some(RefModifier::In));
        } else {
            panic!("expected function");
        }
    }

    #[test]
    fn scoped_identifier() {
        let expr = parse_one_expr("NS::foo");
        if let ExprKind::Identifier(id) = expr.kind {
            assert_eq!(id.scopes, vec!["NS"]);
            assert_eq!(id.name, "foo");
        } else {
            panic!("expected identifier");
        }
    }

    #[test]
    fn init_list_var() {
        let stmt = parse_one_stmt("int[] arr = {1, 2, 3};");
        if let StmtKind::VarDecl(v) = stmt.kind {
            assert!(matches!(v.declarators[0].init, Some(VarInit::InitList(_))));
        } else {
            panic!("expected var decl");
        }
    }

    // ── Error recovery ─────────────────────────────────────────────────

    #[test]
    fn error_recovery_continues() {
        let (script, has_errors) = parse_check("void foo() {} @ void bar() {}");
        // Should still find at least the first function
        assert!(!script.declarations.is_empty());
    }

    // ── Type expression tests ──────────────────────────────────────────

    #[test]
    fn primitive_types_in_function() {
        let decl = parse_one_decl("int foo(float a, double b, bool c) { return 0; }");
        if let Declaration::Function(f) = decl {
            assert_eq!(f.params.len(), 3);
        } else {
            panic!("expected function");
        }
    }

    // ── Complex integration tests ──────────────────────────────────────

    #[test]
    fn full_class_with_methods() {
        let source = r#"
            class Player {
                int health;
                float speed;

                void takeDamage(int amount) {
                    health -= amount;
                    if (health <= 0) {
                        health = 0;
                    }
                }

                bool isAlive() const {
                    return health > 0;
                }
            }
        "#;
        let (script, has_errors) = parse_check(source);
        assert!(!has_errors, "should parse without errors");
        if let Declaration::Class(c) = &script.declarations[0] {
            assert_eq!(c.name, "Player");
            assert_eq!(c.members.len(), 4); // 2 vars + 2 methods
        }
    }

    #[test]
    fn complex_expressions() {
        let source = r#"
            void test() {
                int x = (a + b) * c - d / e;
                bool result = x > 0 && y < 10 || z == 0;
                arr[i] = obj.method(1, 2);
            }
        "#;
        let (_, has_errors) = parse_check(source);
        assert!(!has_errors, "should parse without errors");
    }

    #[test]
    fn enum_and_switch() {
        let source = r#"
            enum State { Idle, Running, Dead }

            void update(State s) {
                switch (s) {
                case Idle:
                    break;
                case Running:
                    move();
                    break;
                default:
                    break;
                }
            }
        "#;
        let (script, has_errors) = parse_check(source);
        assert!(!has_errors, "should parse without errors");
        assert_eq!(script.declarations.len(), 2);
    }

    #[test]
    fn named_arguments() {
        let expr = parse_one_expr("foo(x: 1, y: 2)");
        if let ExprKind::FunctionCall { args, .. } = expr.kind {
            assert_eq!(args[0].name, Some("x".to_string()));
            assert_eq!(args[1].name, Some("y".to_string()));
        } else {
            panic!("expected function call");
        }
    }

    #[test]
    fn destructor_in_class() {
        let decl = parse_one_decl("class Foo { ~Foo() {} }");
        if let Declaration::Class(c) = decl {
            if let ClassMember::Function(f) = &c.members[0] {
                assert!(f.is_destructor);
                assert_eq!(f.name, "Foo");
            } else {
                panic!("expected function member");
            }
        } else {
            panic!("expected class");
        }
    }

    #[test]
    fn string_escape_sequences() {
        let expr = parse_one_expr(r#""hello\nworld""#);
        if let ExprKind::StringLiteral(s) = expr.kind {
            assert_eq!(s, "hello\nworld");
        } else {
            panic!("expected string literal");
        }
    }

    #[test]
    fn bitwise_operators() {
        let expr = parse_one_expr("a & b | c ^ d");
        // Should be ((a & b) | c) ^ d ... actually depends on precedence
        // & (12,13) > ^ (10,11) > | (8,9)
        // So: (a & b) ^ c) | d... no wait:
        // a & b | c ^ d
        // & has highest precedence, then ^, then |
        // So: (a & b) | (c ^ d)
        assert!(matches!(
            expr.kind,
            ExprKind::BinaryOp {
                op: BinOp::BitOr,
                ..
            }
        ));
    }

    #[test]
    fn shift_operators() {
        let expr = parse_one_expr("a << 2");
        assert!(matches!(
            expr.kind,
            ExprKind::BinaryOp {
                op: BinOp::ShiftLeft,
                ..
            }
        ));
    }

    #[test]
    fn default_param() {
        let decl = parse_one_decl("void foo(int x = 5) {}");
        if let Declaration::Function(f) = decl {
            assert!(f.params[0].default.is_some());
        } else {
            panic!("expected function");
        }
    }
}
