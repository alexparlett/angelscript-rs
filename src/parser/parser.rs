use crate::core::error::*;
use crate::core::span::{Span, SpanBuilder};
use crate::parser::ast::*;
use crate::parser::expr_parser::ExprParser;
use crate::parser::token::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    span_builder: Option<SpanBuilder>,
    include_spans: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            span_builder: None,
            include_spans: false,
        }
    }

    pub fn new_with_spans(tokens: Vec<Token>, span_builder: SpanBuilder) -> Self {
        Self {
            tokens,
            pos: 0,
            span_builder: Some(span_builder),
            include_spans: true,
        }
    }

    pub fn parse(mut self) -> ParseResult<Script> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut items = Vec::new();

        while !self.is_at_end() {
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                continue;
            }

            items.push(self.parse_script_item()?);
        }

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Script { items, span })
    }

    fn parse_script_item(&mut self) -> ParseResult<ScriptNode> {
        if self.check(&TokenKind::Hash) {
            return self.parse_directive();
        }

        let start_pos = self.pos;
        while self.identifier_is("shared")
            || self.identifier_is("external")
            || self.identifier_is("final")
            || self.identifier_is("abstract")
        {
            self.advance();
        }

        let t1 = self.current().clone();
        self.pos = start_pos;

        match t1.kind {
            TokenKind::Import => Ok(ScriptNode::Import(self.parse_import()?)),
            TokenKind::Enum => Ok(ScriptNode::Enum(self.parse_enum()?)),
            TokenKind::Typedef => Ok(ScriptNode::Typedef(self.parse_typedef()?)),
            TokenKind::Class => Ok(ScriptNode::Class(self.parse_class()?)),
            TokenKind::Mixin => Ok(ScriptNode::Mixin(self.parse_mixin()?)),
            TokenKind::Interface => Ok(ScriptNode::Interface(self.parse_interface()?)),
            TokenKind::Funcdef => Ok(ScriptNode::FuncDef(self.parse_funcdef()?)),
            TokenKind::Namespace => Ok(ScriptNode::Namespace(self.parse_namespace()?)),
            TokenKind::Semicolon => {
                self.advance();
                self.parse_script_item()
            }
            TokenKind::Const
            | TokenKind::Void
            | TokenKind::Int
            | TokenKind::Int8
            | TokenKind::Int16
            | TokenKind::Int32
            | TokenKind::Int64
            | TokenKind::Uint
            | TokenKind::Uint8
            | TokenKind::Uint16
            | TokenKind::Uint32
            | TokenKind::Uint64
            | TokenKind::Float
            | TokenKind::Double
            | TokenKind::Bool
            | TokenKind::Auto
            | TokenKind::DoubleColon => self.parse_const_or_var_or_func(),
            TokenKind::Identifier(_) => self.parse_const_or_var_or_func(),
            _ => Err(self.error(&format!("Unexpected token: {:?}", t1.kind))),
        }
    }

    fn parse_const_or_var_or_func(&mut self) -> ParseResult<ScriptNode> {
        if self.is_virtual_property_decl() {
            return Ok(ScriptNode::VirtProp(self.parse_virtprop(false, false)?));
        }

        if self.is_func_decl(false) {
            return Ok(ScriptNode::Func(self.parse_function(false)?));
        }

        Ok(ScriptNode::Var(self.parse_var(false, true)?))
    }

    fn parse_directive(&mut self) -> ParseResult<ScriptNode> {
        self.expect(&TokenKind::Hash)?;

        let directive_name = match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            TokenKind::If => {
                self.advance();
                "if".to_string()
            }
            TokenKind::Else => {
                self.advance();
                "else".to_string()
            }
            _ => {
                let name = self.current().lexeme.clone();
                self.advance();
                name
            }
        };

        match directive_name.as_str() {
            "include" => {
                let path = self.expect_string()?;
                Ok(ScriptNode::Include(Include { path }))
            }
            "pragma" => {
                let content = self.read_until_newline();
                Ok(ScriptNode::Pragma(Pragma { content }))
            }
            _ => {
                let content = self.read_until_newline();
                Ok(ScriptNode::CustomDirective(CustomDirective {
                    name: directive_name,
                    content,
                }))
            }
        }
    }

    fn parse_namespace(&mut self) -> ParseResult<Namespace> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Namespace)?;

        let mut name = vec![self.expect_identifier()?];

        while self.check(&TokenKind::DoubleColon) {
            self.advance();
            name.push(self.expect_identifier()?);
        }

        self.expect(&TokenKind::LBrace)?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                continue;
            }
            items.push(self.parse_script_item()?);
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Namespace { name, items, span })
    }

    fn parse_enum(&mut self) -> ParseResult<Enum> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut modifiers = Vec::new();

        while self.identifier_is("shared") || self.identifier_is("external") {
            modifiers.push(self.expect_identifier()?);
        }

        self.expect(&TokenKind::Enum)?;
        let name = self.expect_identifier()?;

        if self.check(&TokenKind::Semicolon) {
            self.advance();
            let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let span = self.make_span(start_offset, end_offset);

            return Ok(Enum {
                modifiers,
                name,
                variants: Vec::new(),
                span,
            });
        }

        self.expect(&TokenKind::LBrace)?;

        let mut variants = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let variant_start = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
            let variant_name = self.expect_identifier()?;
            let value = if self.check(&TokenKind::Assign) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            let variant_end = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let variant_span = self.make_span(variant_start, variant_end);

            variants.push(EnumVariant {
                name: variant_name,
                value,
                span: variant_span,
            });

            if !self.check(&TokenKind::RBrace) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Enum {
            modifiers,
            name,
            variants,
            span,
        })
    }

    fn parse_class(&mut self) -> ParseResult<Class> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut modifiers = Vec::new();

        while self.identifier_is("shared")
            || self.identifier_is("abstract")
            || self.identifier_is("final")
            || self.identifier_is("external")
        {
            modifiers.push(self.expect_identifier()?);
        }

        self.expect(&TokenKind::Class)?;
        let name = self.expect_identifier()?;

        let mut extends = Vec::new();
        if self.check(&TokenKind::Colon) {
            self.advance();
            extends.push(self.expect_identifier()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                extends.push(self.expect_identifier()?);
            }
        }

        if self.check(&TokenKind::Semicolon) {
            self.advance();
            let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let span = self.make_span(start_offset, end_offset);

            return Ok(Class {
                modifiers,
                name,
                extends,
                members: Vec::new(),
                span,
            });
        }

        self.expect(&TokenKind::LBrace)?;

        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                continue;
            }
            members.push(self.parse_class_member(&name)?);
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Class {
            modifiers,
            name,
            extends,
            members,
            span,
        })
    }

    fn parse_class_member(&mut self, class_name: &str) -> ParseResult<ClassMember> {
        if self.check(&TokenKind::BitNot) {
            return Ok(ClassMember::Func(self.parse_function(true)?));
        }

        if let TokenKind::Identifier(name) = &self.current().kind {
            if name == class_name {
                let next_pos = self.pos + 1;
                if next_pos < self.tokens.len() && self.tokens[next_pos].kind == TokenKind::LParen {
                    return Ok(ClassMember::Func(self.parse_function(true)?));
                }
            }
        }

        if self.check(&TokenKind::Funcdef) {
            return Ok(ClassMember::FuncDef(self.parse_funcdef()?));
        }

        if self.is_func_decl(true) {
            Ok(ClassMember::Func(self.parse_function(true)?))
        } else if self.is_virtual_property_decl() {
            Ok(ClassMember::VirtProp(self.parse_virtprop(true, false)?))
        } else if self.is_var_decl() {
            Ok(ClassMember::Var(self.parse_var(true, false)?))
        } else {
            Err(self.error("Expected class member"))
        }
    }

    fn parse_interface(&mut self) -> ParseResult<Interface> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut modifiers = Vec::new();

        while self.identifier_is("shared") || self.identifier_is("external") {
            modifiers.push(self.expect_identifier()?);
        }

        self.expect(&TokenKind::Interface)?;
        let name = self.expect_identifier()?;

        let mut extends = Vec::new();
        if self.check(&TokenKind::Colon) {
            self.advance();
            extends.push(self.expect_identifier()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                extends.push(self.expect_identifier()?);
            }
        }

        if self.check(&TokenKind::Semicolon) {
            self.advance();
            let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let span = self.make_span(start_offset, end_offset);

            return Ok(Interface {
                modifiers,
                name,
                extends,
                members: Vec::new(),
                span,
            });
        }

        self.expect(&TokenKind::LBrace)?;

        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                continue;
            }

            if self.is_virtual_property_decl() {
                members.push(InterfaceMember::VirtProp(self.parse_virtprop(true, true)?));
            } else {
                members.push(InterfaceMember::Method(self.parse_interface_method()?));
            }
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Interface {
            modifiers,
            name,
            extends,
            members,
            span,
        })
    }

    fn parse_interface_method(&mut self) -> ParseResult<IntfMthd> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        let return_type = self.parse_type()?;
        let is_ref = if self.check(&TokenKind::BitAnd) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;
        let params = self.parse_param_list()?;

        let is_const = if self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(IntfMthd {
            return_type,
            is_ref,
            name,
            params,
            is_const,
            span,
        })
    }

    fn parse_typedef(&mut self) -> ParseResult<Typedef> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Typedef)?;

        let prim_type = match &self.current().kind {
            TokenKind::Int | TokenKind::Int32 => "int",
            TokenKind::Int8 => "int8",
            TokenKind::Int16 => "int16",
            TokenKind::Int64 => "int64",
            TokenKind::Uint | TokenKind::Uint32 => "uint",
            TokenKind::Uint8 => "uint8",
            TokenKind::Uint16 => "uint16",
            TokenKind::Uint64 => "uint64",
            TokenKind::Float => "float",
            TokenKind::Double => "double",
            TokenKind::Bool => "bool",
            _ => return Err(self.error("Expected primitive type")),
        };

        let prim_type = prim_type.to_string();
        self.advance();

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Typedef {
            prim_type,
            name,
            span,
        })
    }

    fn parse_funcdef(&mut self) -> ParseResult<FuncDef> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut modifiers = Vec::new();

        while self.identifier_is("shared") || self.identifier_is("external") {
            modifiers.push(self.expect_identifier()?);
        }

        self.expect(&TokenKind::Funcdef)?;

        let return_type = self.parse_type()?;
        let is_ref = if self.check(&TokenKind::BitAnd) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;
        let params = self.parse_param_list()?;

        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(FuncDef {
            modifiers,
            return_type,
            is_ref,
            name,
            params,
            span,
        })
    }

    fn parse_mixin(&mut self) -> ParseResult<Mixin> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Mixin)?;
        let class = self.parse_class()?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Mixin { class, span })
    }

    fn parse_import(&mut self) -> ParseResult<Import> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Import)?;

        let type_name = self.parse_type()?;
        let is_ref = if self.check(&TokenKind::BitAnd) {
            self.advance();
            true
        } else {
            false
        };

        let identifier = self.expect_identifier()?;
        let params = self.parse_param_list()?;

        while self.is_func_attribute() {
            self.advance();
        }

        self.expect_contextual_keyword("from")?;
        let from = self.expect_string()?;
        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Import {
            type_name,
            is_ref,
            identifier,
            params,
            from,
            span,
        })
    }

    fn parse_virtprop(&mut self, is_method: bool, is_interface: bool) -> ParseResult<VirtProp> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut visibility = None;

        if is_method {
            if self.check(&TokenKind::Private) {
                visibility = Some(Visibility::Private);
                self.advance();
            } else if self.check(&TokenKind::Protected) {
                visibility = Some(Visibility::Protected);
                self.advance();
            }
        }

        let prop_type = self.parse_type()?;
        let is_ref = if self.check(&TokenKind::BitAnd) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut accessors = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let accessor_start = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

            let kind = if self.identifier_is("get") {
                self.advance();
                AccessorKind::Get
            } else if self.identifier_is("set") {
                self.advance();
                AccessorKind::Set
            } else {
                return Err(self.error("Expected 'get' or 'set'"));
            };

            let is_const = if self.check(&TokenKind::Const) {
                self.advance();
                true
            } else {
                false
            };

            let mut attributes = Vec::new();
            if is_method && !is_interface {
                attributes = self.parse_func_attributes()?;
            }

            let body = if !is_interface {
                if self.check(&TokenKind::LBrace) {
                    Some(self.parse_stat_block()?)
                } else {
                    self.expect(&TokenKind::Semicolon)?;
                    None
                }
            } else {
                self.expect(&TokenKind::Semicolon)?;
                None
            };

            let accessor_end = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let accessor_span = self.make_span(accessor_start, accessor_end);

            accessors.push(PropertyAccessor {
                kind,
                is_const,
                attributes,
                body,
                span: accessor_span,
            });
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(VirtProp {
            visibility,
            prop_type,
            is_ref,
            name,
            accessors,
            span,
        })
    }

    fn parse_function(&mut self, is_method: bool) -> ParseResult<Func> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut modifiers = Vec::new();
        let mut visibility = None;

        if !is_method {
            while self.identifier_is("shared") || self.identifier_is("external") {
                modifiers.push(self.expect_identifier()?);
            }
        }

        if is_method {
            if self.check(&TokenKind::Private) {
                visibility = Some(Visibility::Private);
                self.advance();
            } else if self.check(&TokenKind::Protected) {
                visibility = Some(Visibility::Protected);
                self.advance();
            }
        }

        if self.check(&TokenKind::BitNot) {
            self.advance();
            let name = format!("~{}", self.expect_identifier()?);
            let params = self.parse_param_list()?;
            let attributes = self.parse_func_attributes()?;

            let body = if self.check(&TokenKind::LBrace) {
                Some(self.parse_stat_block()?)
            } else {
                self.expect(&TokenKind::Semicolon)?;
                None
            };

            let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let span = self.make_span(start_offset, end_offset);

            return Ok(Func {
                modifiers,
                visibility,
                return_type: None,
                is_ref: false,
                name,
                params,
                is_const: false,
                attributes,
                body,
                span,
            });
        }

        let mut return_type = None;
        let mut is_ref = false;
        let name;

        if is_method {
            let checkpoint = self.pos;

            if self.check_identifier() {
                let potential_name = self.expect_identifier()?;

                if self.check(&TokenKind::LParen) {
                    name = potential_name;
                } else {
                    self.pos = checkpoint;
                    return_type = Some(self.parse_type()?);
                    is_ref = if self.check(&TokenKind::BitAnd) {
                        self.advance();
                        true
                    } else {
                        false
                    };
                    name = self.expect_identifier()?;
                }
            } else {
                return_type = Some(self.parse_type()?);
                is_ref = if self.check(&TokenKind::BitAnd) {
                    self.advance();
                    true
                } else {
                    false
                };
                name = self.expect_identifier()?;
            }
        } else {
            return_type = Some(self.parse_type()?);
            is_ref = if self.check(&TokenKind::BitAnd) {
                self.advance();
                true
            } else {
                false
            };
            name = self.expect_identifier()?;
        }

        let params = self.parse_param_list()?;

        let is_const = if is_method && self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        let attributes = self.parse_func_attributes()?;

        let body = if self.check(&TokenKind::Semicolon) {
            self.advance();
            None
        } else if self.check(&TokenKind::LBrace) {
            Some(self.parse_stat_block()?)
        } else {
            return Err(self.error("Expected ';' or '{'"));
        };

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Func {
            modifiers,
            visibility,
            return_type,
            is_ref,
            name,
            params,
            is_const,
            attributes,
            body,
            span,
        })
    }

    pub(crate) fn parse_var(
        &mut self,
        is_class_prop: bool,
        is_global_var: bool,
    ) -> ParseResult<Var> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut visibility = None;

        if is_class_prop {
            if self.check(&TokenKind::Private) {
                visibility = Some(Visibility::Private);
                self.advance();
            } else if self.check(&TokenKind::Protected) {
                visibility = Some(Visibility::Protected);
                self.advance();
            }
        }

        let var_type = self.parse_type()?;

        let mut declarations = Vec::new();

        loop {
            let decl_start = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

            if self.check(&TokenKind::At) {
                self.advance();
            }

            let name = self.expect_identifier()?;

            let initializer = if is_class_prop || is_global_var {
                if self.check(&TokenKind::Assign) || self.check(&TokenKind::LParen) {
                    Some(self.superficially_parse_var_init()?)
                } else {
                    None
                }
            } else {
                if self.check(&TokenKind::LParen) {
                    Some(VarInit::ArgList(self.parse_arg_list()?))
                } else if self.check(&TokenKind::Assign) {
                    self.advance();
                    if self.check(&TokenKind::LBrace) {
                        Some(VarInit::InitList(self.parse_init_list()?))
                    } else {
                        Some(VarInit::Expr(self.parse_expression()?))
                    }
                } else {
                    None
                }
            };

            let decl_end = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let decl_span = self.make_span(decl_start, decl_end);

            declarations.push(VarDecl {
                name,
                initializer,
                span: decl_span,
            });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Var {
            visibility,
            var_type,
            declarations,
            span,
        })
    }

    fn superficially_parse_var_init(&mut self) -> ParseResult<VarInit> {
        if self.check(&TokenKind::Assign) {
            self.advance();

            let mut depth_paren = 0;
            let mut depth_brace = 0;

            while !self.is_at_end() {
                if self.check(&TokenKind::LParen) {
                    depth_paren += 1;
                    self.advance();
                } else if self.check(&TokenKind::RParen) {
                    if depth_paren == 0 {
                        break;
                    }
                    depth_paren -= 1;
                    self.advance();
                } else if self.check(&TokenKind::LBrace) {
                    depth_brace += 1;
                    self.advance();
                } else if self.check(&TokenKind::RBrace) {
                    if depth_brace == 0 {
                        break;
                    }
                    depth_brace -= 1;
                    self.advance();
                } else if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                    if depth_paren == 0 && depth_brace == 0 {
                        break;
                    }
                    self.advance();
                } else {
                    self.advance();
                }
            }

            Ok(VarInit::Expr(Expr::Void(None)))
        } else if self.check(&TokenKind::LParen) {
            let mut depth = 1;
            self.advance();

            while depth > 0 && !self.is_at_end() {
                if self.check(&TokenKind::LParen) {
                    depth += 1;
                } else if self.check(&TokenKind::RParen) {
                    depth -= 1;
                }
                self.advance();
            }

            Ok(VarInit::ArgList(Vec::new()))
        } else {
            Err(self.error("Expected '=' or '('"))
        }
    }

    pub fn parse_type(&mut self) -> ParseResult<Type> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        let is_const = if self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        let scope = self.parse_scope()?;
        let datatype = self.parse_datatype()?;

        let template_types = if self.check(&TokenKind::Lt) {
            self.parse_template_args()?
        } else {
            Vec::new()
        };

        let mut modifiers = Vec::new();

        loop {
            if self.check(&TokenKind::LBracket) {
                self.advance();
                self.expect(&TokenKind::RBracket)?;
                modifiers.push(TypeModifier::Array);
            } else if self.check(&TokenKind::At) {
                self.advance();
                if self.check(&TokenKind::Const) {
                    self.advance();
                    modifiers.push(TypeModifier::ConstHandle);
                } else {
                    modifiers.push(TypeModifier::Handle);
                }
            } else {
                break;
            }
        }

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Type {
            is_const,
            scope,
            datatype,
            template_types,
            modifiers,
            span,
        })
    }

    fn parse_scope(&mut self) -> ParseResult<Scope> {
        let mut is_global = false;
        let mut path = Vec::new();

        if self.check(&TokenKind::DoubleColon) {
            is_global = true;
            self.advance();
        }

        while self.check_identifier() {
            let checkpoint = self.pos;
            let ident = self.expect_identifier()?;

            if self.check(&TokenKind::DoubleColon) {
                self.advance();
                path.push(ident);
            } else {
                self.pos = checkpoint;
                break;
            }
        }

        Ok(Scope { is_global, path })
    }

    fn parse_datatype(&mut self) -> ParseResult<DataType> {
        match &self.current().kind {
            TokenKind::Void => {
                self.advance();
                Ok(DataType::PrimType("void".to_string()))
            }
            TokenKind::Int | TokenKind::Int32 => {
                self.advance();
                Ok(DataType::PrimType("int".to_string()))
            }
            TokenKind::Int8 => {
                self.advance();
                Ok(DataType::PrimType("int8".to_string()))
            }
            TokenKind::Int16 => {
                self.advance();
                Ok(DataType::PrimType("int16".to_string()))
            }
            TokenKind::Int64 => {
                self.advance();
                Ok(DataType::PrimType("int64".to_string()))
            }
            TokenKind::Uint | TokenKind::Uint32 => {
                self.advance();
                Ok(DataType::PrimType("uint".to_string()))
            }
            TokenKind::Uint8 => {
                self.advance();
                Ok(DataType::PrimType("uint8".to_string()))
            }
            TokenKind::Uint16 => {
                self.advance();
                Ok(DataType::PrimType("uint16".to_string()))
            }
            TokenKind::Uint64 => {
                self.advance();
                Ok(DataType::PrimType("uint64".to_string()))
            }
            TokenKind::Float => {
                self.advance();
                Ok(DataType::PrimType("float".to_string()))
            }
            TokenKind::Double => {
                self.advance();
                Ok(DataType::PrimType("double".to_string()))
            }
            TokenKind::Bool => {
                self.advance();
                Ok(DataType::PrimType("bool".to_string()))
            }
            TokenKind::Auto => {
                self.advance();
                Ok(DataType::Auto)
            }
            TokenKind::Question => {
                self.advance();
                Ok(DataType::Question)
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(DataType::Identifier(name))
            }
            _ => Err(self.error("Expected type")),
        }
    }

    fn parse_template_args(&mut self) -> ParseResult<Vec<Type>> {
        self.expect(&TokenKind::Lt)?;

        let mut types = vec![self.parse_type()?];

        while self.check(&TokenKind::Comma) {
            self.advance();
            types.push(self.parse_type()?);
        }

        self.expect_gt_in_template()?;

        Ok(types)
    }

    fn parse_param_list(&mut self) -> ParseResult<Vec<Param>> {
        self.expect(&TokenKind::LParen)?;

        if self.check(&TokenKind::Void) {
            self.advance();
            self.expect(&TokenKind::RParen)?;
            return Ok(Vec::new());
        }

        if self.check(&TokenKind::RParen) {
            self.advance();
            return Ok(Vec::new());
        }

        let mut params = vec![self.parse_param()?];

        while self.check(&TokenKind::Comma) {
            self.advance();
            params.push(self.parse_param()?);
        }

        self.expect(&TokenKind::RParen)?;

        Ok(params)
    }

    fn parse_param(&mut self) -> ParseResult<Param> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        let param_type = self.parse_type()?;

        let mut type_mod = None;

        if self.check(&TokenKind::BitAnd) {
            self.advance();

            type_mod = Some(if self.check(&TokenKind::In) {
                self.advance();
                TypeMod::In
            } else if self.check(&TokenKind::Out) {
                self.advance();
                TypeMod::Out
            } else if self.check(&TokenKind::InOut) {
                self.advance();
                TypeMod::InOut
            } else {
                TypeMod::InOut
            });
        }

        let name = if self.check_identifier() {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        let default_value = if self.check(&TokenKind::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Param {
            param_type,
            type_mod,
            name,
            default_value,
            is_variadic: false,
            span,
        })
    }

    fn parse_func_attributes(&mut self) -> ParseResult<Vec<String>> {
        let mut attributes = Vec::new();

        while self.is_func_attribute() {
            attributes.push(self.expect_identifier()?);
        }

        Ok(attributes)
    }

    fn is_func_attribute(&self) -> bool {
        if let TokenKind::Identifier(name) = &self.current().kind {
            matches!(
                name.as_str(),
                "override" | "final" | "explicit" | "property" | "delete"
            )
        } else {
            false
        }
    }

    fn parse_stat_block(&mut self) -> ParseResult<StatBlock> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::LBrace)?;

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.is_var_decl() {
                statements.push(Statement::Var(self.parse_var(false, false)?));
            } else {
                statements.push(self.parse_statement()?);
            }
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(StatBlock { statements, span })
    }

    fn try_parse_type(&mut self) -> ParseResult<Type> {
        self.parse_type()
    }

    fn is_virtual_property_decl(&mut self) -> bool {
        let checkpoint = self.pos;

        if matches!(
            self.current().kind,
            TokenKind::Private | TokenKind::Protected
        ) {
            self.advance();
        }

        if self.try_parse_type().is_err() {
            self.pos = checkpoint;
            return false;
        }

        if !self.check_identifier() {
            self.pos = checkpoint;
            return false;
        }

        self.advance();

        let result = self.check(&TokenKind::LBrace);

        self.pos = checkpoint;
        result
    }

    fn is_func_decl(&mut self, is_method: bool) -> bool {
        let checkpoint = self.pos;

        if is_method {
            if self.check(&TokenKind::BitNot) {
                self.pos = checkpoint;
                return true;
            }

            if matches!(
                self.current().kind,
                TokenKind::Private | TokenKind::Protected
            ) {
                self.advance();
            }

            if self.check_identifier() {
                self.advance();
                if self.check(&TokenKind::LParen) {
                    self.pos = checkpoint;
                    return true;
                }
                self.pos = checkpoint;
            }
        }

        if self.check(&TokenKind::Const) {
            self.advance();
        }

        if self.check(&TokenKind::DoubleColon) {
            self.advance();
        }

        while self.check_identifier() {
            self.advance();
            if self.check(&TokenKind::DoubleColon) {
                self.advance();
            } else {
                break;
            }
        }

        if !self.is_type_token() {
            self.pos = checkpoint;
            return false;
        }
        self.advance();

        if self.check(&TokenKind::Lt) {
            if !self.skip_template_args() {
                self.pos = checkpoint;
                return false;
            }
        }

        while self.check(&TokenKind::LBracket) || self.check(&TokenKind::At) {
            if self.check(&TokenKind::LBracket) {
                self.advance();
                if self.check(&TokenKind::RBracket) {
                    self.advance();
                }
            } else {
                self.advance();
                if self.check(&TokenKind::Const) {
                    self.advance();
                }
            }
        }

        if self.check(&TokenKind::BitAnd) {
            self.advance();
        }

        if !self.check_identifier() {
            self.pos = checkpoint;
            return false;
        }
        self.advance();

        let result = self.check(&TokenKind::LParen);
        self.pos = checkpoint;
        result
    }

    fn is_var_decl(&mut self) -> bool {
        let checkpoint = self.pos;

        if matches!(
            self.current().kind,
            TokenKind::Private | TokenKind::Protected
        ) {
            self.advance();
        }

        if self.check(&TokenKind::Const) {
            self.advance();
        }

        if self.check(&TokenKind::At) {
            self.pos = checkpoint;
            return false;
        }

        if self.check(&TokenKind::DoubleColon) {
            self.advance();
        }
        while self.check_identifier() {
            let next_pos = self.pos + 1;
            if next_pos < self.tokens.len() && self.tokens[next_pos].kind == TokenKind::DoubleColon
            {
                self.advance();
                self.advance();
            } else {
                break;
            }
        }

        if !self.is_type_token() {
            self.pos = checkpoint;
            return false;
        }
        self.advance();

        if self.check(&TokenKind::Lt) {
            if !self.skip_template_args() {
                self.pos = checkpoint;
                return false;
            }
        }

        while self.check(&TokenKind::LBracket) || self.check(&TokenKind::At) {
            self.advance();
            if self.check(&TokenKind::RBracket) {
                self.advance();
            }
            if self.check(&TokenKind::Const) {
                self.advance();
            }
        }

        if !self.check_identifier() {
            self.pos = checkpoint;
            return false;
        }
        self.advance();

        let result = self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::Assign)
            || self.check(&TokenKind::Comma)
            || self.check(&TokenKind::LParen);

        self.pos = checkpoint;
        result
    }

    fn is_type_token(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Void
                | TokenKind::Int
                | TokenKind::Int8
                | TokenKind::Int16
                | TokenKind::Int32
                | TokenKind::Int64
                | TokenKind::Uint
                | TokenKind::Uint8
                | TokenKind::Uint16
                | TokenKind::Uint32
                | TokenKind::Uint64
                | TokenKind::Float
                | TokenKind::Double
                | TokenKind::Bool
                | TokenKind::Auto
                | TokenKind::Identifier(_)
        )
    }

    fn skip_template_args(&mut self) -> bool {
        if !self.check(&TokenKind::Lt) {
            return false;
        }

        let mut depth: i8 = 1;
        self.advance();

        while depth > 0 && !self.is_at_end() {
            match &self.current().kind {
                TokenKind::Lt => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Gt => {
                    depth -= 1;
                    self.advance();
                }
                TokenKind::Shr => {
                    depth = depth.saturating_sub(2);
                    self.advance();
                }
                TokenKind::UShr => {
                    depth = depth.saturating_sub(3);
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        depth == 0
    }

    fn expect_gt_in_template(&mut self) -> ParseResult<()> {
        match &self.current().kind {
            TokenKind::Gt => {
                self.advance();
                Ok(())
            }
            TokenKind::Shr => {
                let current_span = self.current().span.clone();
                let lexeme = ">".to_string();
                let new_span = current_span.as_ref().map(|s| {
                    Span::new(
                        s.source_name.clone(),
                        s.start + 1,
                        s.end,
                        s.start_line,
                        s.start_column + 1,
                        s.end_line,
                        s.end_column,
                    )
                });

                let gt_token = Token::new(TokenKind::Gt, new_span, lexeme);
                self.tokens[self.pos] = gt_token;
                Ok(())
            }
            TokenKind::UShr => {
                let current_span = self.current().span.clone();
                let lexeme = ">>".to_string();
                let new_span = current_span.as_ref().map(|s| {
                    Span::new(
                        s.source_name.clone(),
                        s.start + 1,
                        s.end,
                        s.start_line,
                        s.start_column + 1,
                        s.end_line,
                        s.end_column,
                    )
                });

                let shr_token = Token::new(TokenKind::Shr, new_span, lexeme);
                self.tokens[self.pos] = shr_token;
                Ok(())
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "'>'".to_string(),
                found: format!("{:?}", self.current().kind),
                span: self.current().span.clone(),
            }),
        }
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        match &self.current().kind {
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::ForEach => self.parse_foreach(),
            TokenKind::While => self.parse_while(),
            TokenKind::Do => self.parse_do_while(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => {
                let span = self.current().span.clone();
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(Statement::Break(span))
            }
            TokenKind::Continue => {
                let span = self.current().span.clone();
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(Statement::Continue(span))
            }
            TokenKind::Try => self.parse_try(),
            TokenKind::LBrace => Ok(Statement::Block(self.parse_stat_block()?)),
            _ => self.parse_expr_statement(),
        }
    }

    fn parse_foreach(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::ForEach)?;
        self.expect(&TokenKind::LParen)?;

        let mut variables = Vec::new();

        loop {
            let var_type = self.parse_type()?;
            let var_name = self.expect_identifier()?;

            variables.push((var_type, var_name));

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else if self.check(&TokenKind::Colon) {
                break;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "',' or ':'".to_string(),
                    found: format!("{:?}", self.current().kind),
                    span: self.current().span.clone(),
                });
            }
        }

        self.expect(&TokenKind::Colon)?;

        let iterable = self.parse_expression()?;

        self.expect(&TokenKind::RParen)?;

        let body = Box::new(self.parse_statement()?);

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::ForEach(ForEachStmt {
            variables,
            iterable,
            body,
            span,
        }))
    }

    fn parse_if(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::If)?;
        self.expect(&TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;

        let then_branch = Box::new(self.parse_statement()?);

        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::If(IfStmt {
            condition,
            then_branch,
            else_branch,
            span,
        }))
    }

    fn parse_for(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::For)?;
        self.expect(&TokenKind::LParen)?;

        let init = if self.is_var_decl() {
            ForInit::Var(self.parse_var(false, false)?)
        } else {
            ForInit::Expr(self.parse_expr_statement_inner()?)
        };

        let condition = self.parse_expr_statement_inner()?;

        let mut increment = Vec::new();
        if !self.check(&TokenKind::RParen) {
            increment.push(self.parse_expression()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                increment.push(self.parse_expression()?);
            }
        }

        self.expect(&TokenKind::RParen)?;

        let body = Box::new(self.parse_statement()?);

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::For(ForStmt {
            init,
            condition,
            increment,
            body,
            span,
        }))
    }

    fn parse_while(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;

        let body = Box::new(self.parse_statement()?);

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::While(WhileStmt {
            condition,
            body,
            span,
        }))
    }

    fn parse_do_while(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Do)?;
        let body = Box::new(self.parse_statement()?);
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::DoWhile(DoWhileStmt {
            body,
            condition,
            span,
        }))
    }

    fn parse_switch(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Switch)?;
        self.expect(&TokenKind::LParen)?;
        let value = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::LBrace)?;

        let mut cases = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            cases.push(self.parse_case()?);
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::Switch(SwitchStmt { value, cases, span }))
    }

    fn parse_case(&mut self) -> ParseResult<Case> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        let pattern = if self.check(&TokenKind::Case) {
            self.advance();
            CasePattern::Value(self.parse_expression()?)
        } else if self.check(&TokenKind::Default) {
            self.advance();
            CasePattern::Default
        } else {
            return Err(self.error("Expected 'case' or 'default'"));
        };

        self.expect(&TokenKind::Colon)?;

        let mut statements = Vec::new();

        while !matches!(
            self.current().kind,
            TokenKind::Case | TokenKind::Default | TokenKind::RBrace
        ) && !self.is_at_end()
        {
            if self.is_var_decl() {
                statements.push(Statement::Var(self.parse_var(false, false)?));
            } else {
                statements.push(self.parse_statement()?);
            }
        }

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Case {
            pattern,
            statements,
            span,
        })
    }

    fn parse_return(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Return)?;

        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(&TokenKind::Semicolon)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::Return(ReturnStmt { value, span }))
    }

    fn parse_try(&mut self) -> ParseResult<Statement> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::Try)?;
        let try_block = self.parse_stat_block()?;
        self.expect(&TokenKind::Catch)?;
        let catch_block = self.parse_stat_block()?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Statement::Try(TryStmt {
            try_block,
            catch_block,
            span,
        }))
    }

    fn parse_expr_statement(&mut self) -> ParseResult<Statement> {
        let expr = self.parse_expr_statement_inner()?;
        Ok(Statement::Expr(expr))
    }

    fn parse_expr_statement_inner(&mut self) -> ParseResult<Option<Expr>> {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
            return Ok(None);
        }

        let expr = self.parse_expression()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(Some(expr))
    }

    fn parse_expression(&mut self) -> ParseResult<Expr> {
        let expr_tokens = self.collect_expression_tokens()?;

        if expr_tokens.is_empty() {
            return Err(self.error("Expected expression"));
        }

        let pratt = ExprParser::new(expr_tokens, self.span_builder.clone(), self.include_spans);
        pratt.parse()
    }

    fn collect_expression_tokens(&mut self) -> ParseResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut paren_depth = 0;
        let mut bracket_depth = 0;
        let mut brace_depth = 0;
        let mut angle_depth: i32 = 0;

        loop {
            let token = self.current().clone();

            match &token.kind {
                TokenKind::LParen => {
                    paren_depth += 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::RParen => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::LBracket => {
                    bracket_depth += 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::RBracket => {
                    if bracket_depth == 0 {
                        break;
                    }
                    bracket_depth -= 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::LBrace => {
                    brace_depth += 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::RBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Lt => {
                    let looks_like_template = self.looks_like_template_lookahead(&tokens);

                    if looks_like_template {
                        angle_depth += 1;
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Gt => {
                    if angle_depth > 0 {
                        angle_depth -= 1;
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Shr => {
                    angle_depth = angle_depth.saturating_sub(2);
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::UShr => {
                    angle_depth = angle_depth.saturating_sub(3);
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Semicolon => {
                    if paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                        && angle_depth == 0
                    {
                        break;
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Comma => {
                    if paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                        && angle_depth == 0
                    {
                        break;
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Colon => {
                    if paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                        && angle_depth == 0
                    {
                        let has_question = tokens.iter().any(|t| t.kind == TokenKind::Question);
                        if !has_question {
                            break;
                        }
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Eof => break,
                _ => {
                    tokens.push(token);
                    self.advance();
                }
            }
        }

        Ok(tokens)
    }

    fn looks_like_template_lookahead(&self, tokens_so_far: &[Token]) -> bool {
        if tokens_so_far.is_empty() {
            return false;
        }
        if !matches!(tokens_so_far.last().unwrap().kind, TokenKind::Identifier(_)) {
            return false;
        }

        let mut scan_pos = self.pos + 1;
        let mut depth: i32 = 1;
        let mut saw_type = false;

        while scan_pos < self.tokens.len() && depth > 0 {
            match &self.tokens[scan_pos].kind {
                TokenKind::Identifier(_)
                | TokenKind::Const
                | TokenKind::Int
                | TokenKind::Int8
                | TokenKind::Int16
                | TokenKind::Int32
                | TokenKind::Int64
                | TokenKind::Uint
                | TokenKind::Uint8
                | TokenKind::Uint16
                | TokenKind::Uint32
                | TokenKind::Uint64
                | TokenKind::Float
                | TokenKind::Double
                | TokenKind::Bool
                | TokenKind::Void => {
                    saw_type = true;
                    scan_pos += 1;
                }
                TokenKind::Lt => {
                    depth += 1;
                    scan_pos += 1;
                }
                TokenKind::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        return saw_type;
                    }
                    scan_pos += 1;
                }
                TokenKind::Shr => {
                    if depth >= 2 {
                        depth -= 2;
                    } else {
                        depth = 0;
                    }
                    if depth == 0 {
                        return saw_type;
                    }
                    scan_pos += 1;
                }
                TokenKind::UShr => {
                    depth = depth.saturating_sub(3);
                    if depth == 0 {
                        return saw_type;
                    }
                    scan_pos += 1;
                }
                TokenKind::Comma
                | TokenKind::DoubleColon
                | TokenKind::LBracket
                | TokenKind::RBracket
                | TokenKind::At => {
                    scan_pos += 1;
                }
                TokenKind::Add
                | TokenKind::Sub
                | TokenKind::Mul
                | TokenKind::Div
                | TokenKind::Mod
                | TokenKind::Eq
                | TokenKind::Ne
                | TokenKind::Le
                | TokenKind::Ge
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Xor
                | TokenKind::BitAnd
                | TokenKind::BitOr
                | TokenKind::BitXor
                | TokenKind::Shl
                | TokenKind::Assign
                | TokenKind::Number(_) => {
                    return false;
                }
                _ => return false,
            }

            if scan_pos - self.pos > 100 {
                return false;
            }
        }

        false
    }

    fn parse_init_list(&mut self) -> ParseResult<InitList> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);

        self.expect(&TokenKind::LBrace)?;

        let mut items = Vec::new();

        if !self.check(&TokenKind::RBrace) {
            loop {
                if self.check(&TokenKind::LBrace) {
                    items.push(InitListItem::InitList(self.parse_init_list()?));
                } else {
                    items.push(InitListItem::Expr(self.parse_expression()?));
                }

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();

                if self.check(&TokenKind::RBrace) {
                    break;
                }
            }
        }

        self.expect(&TokenKind::RBrace)?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(InitList { items, span })
    }

    fn parse_arg_list(&mut self) -> ParseResult<Vec<Arg>> {
        self.expect(&TokenKind::LParen)?;

        if self.check(&TokenKind::RParen) {
            self.advance();
            return Ok(Vec::new());
        }

        let mut args = Vec::new();

        loop {
            let arg_start = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
            let value = self.parse_expression()?;
            let arg_end = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
            let arg_span = self.make_span(arg_start, arg_end);

            args.push(Arg {
                name: None,
                value,
                span: arg_span,
            });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.expect(&TokenKind::RParen)?;

        Ok(args)
    }

    fn identifier_is(&self, name: &str) -> bool {
        if let TokenKind::Identifier(id) = &self.current().kind {
            id == name
        } else {
            false
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn previous(&self) -> &Token {
        if self.pos > 0 {
            &self.tokens[self.pos - 1]
        } else {
            &self.tokens[0]
        }
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.current().kind, TokenKind::Identifier(_))
    }

    fn expect(&mut self, kind: &TokenKind) -> ParseResult<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", kind),
                found: format!("{:?}", self.current().kind),
                span: self.current().span.clone(),
            })
        }
    }

    fn expect_identifier(&mut self) -> ParseResult<String> {
        match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("Expected identifier")),
        }
    }

    fn expect_string(&mut self) -> ParseResult<String> {
        if let TokenKind::String(s) = &self.current().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(self.error("Expected string literal"))
        }
    }

    fn expect_contextual_keyword(&mut self, keyword: &str) -> ParseResult<()> {
        if self.identifier_is(keyword) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("Expected keyword '{}'", keyword)))
        }
    }

    fn read_until_newline(&mut self) -> String {
        let mut content = String::new();
        let start_line = self.current().line();

        while !self.is_at_end() && self.current().line() == start_line {
            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(&self.current().lexeme);
            self.advance();
        }

        content.trim().to_string()
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError::SyntaxError {
            message: message.to_string(),
            span: self.current().span.clone(),
        }
    }

    fn make_span(&self, start: usize, end: usize) -> Option<Span> {
        if self.include_spans {
            self.span_builder.as_ref().map(|sb| sb.span(start, end))
        } else {
            None
        }
    }

    pub fn parse_function_signature(&mut self) -> ParseResult<Func> {
        let start_offset = self.current().span.as_ref().map(|s| s.start).unwrap_or(0);
        let mut modifiers = Vec::new();
        let mut visibility = None;

        while self.identifier_is("shared")
            || self.identifier_is("external")
            || self.identifier_is("final")
            || self.identifier_is("override")
            || self.identifier_is("virtual")
            || self.identifier_is("explicit")
        {
            modifiers.push(self.expect_identifier()?);
        }

        if self.check(&TokenKind::Private) {
            visibility = Some(Visibility::Private);
            self.advance();
        } else if self.check(&TokenKind::Protected) {
            visibility = Some(Visibility::Protected);
            self.advance();
        }

        let (return_type, is_ref) = if self.is_type_token() || self.check(&TokenKind::Const) {
            let ret_type = self.parse_type()?;
            let is_ref = if self.check(&TokenKind::BitAnd) {
                self.advance();
                true
            } else {
                false
            };
            (Some(ret_type), is_ref)
        } else {
            (None, false)
        };

        let name = self.expect_identifier()?;
        let params = self.parse_param_list()?;

        let is_const = if self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        let attributes = self.parse_func_attributes()?;

        let end_offset = self.previous().span.as_ref().map(|s| s.end).unwrap_or(0);
        let span = self.make_span(start_offset, end_offset);

        Ok(Func {
            modifiers,
            visibility,
            return_type,
            is_ref,
            name,
            params,
            is_const,
            attributes,
            body: None,
            span,
        })
    }
}
