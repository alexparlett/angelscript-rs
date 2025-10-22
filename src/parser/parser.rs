use crate::parser::ast::*;
use crate::parser::error::*;
use crate::parser::expr_parser::ExprParser;
use crate::parser::token::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(mut self) -> Result<Script> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                continue;
            }

            items.push(self.parse_script_item()?);
        }

        Ok(Script { items })
    }

    fn parse_script_item(&mut self) -> Result<ScriptNode> {
        // Handle preprocessor directives
        if self.check(&TokenKind::Hash) {
            return self.parse_directive();
        }

        // Optimize by skipping tokens 'shared', 'external', 'final', 'abstract'
        let start_pos = self.pos;
        while self.identifier_is("shared")
            || self.identifier_is("external")
            || self.identifier_is("final")
            || self.identifier_is("abstract")
        {
            self.advance();
        }

        let t1 = self.current().clone();
        self.pos = start_pos; // Rewind

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

    fn parse_const_or_var_or_func(&mut self) -> Result<ScriptNode> {
        if self.is_virtual_property_decl() {
            return Ok(ScriptNode::VirtProp(self.parse_virtprop(false, false)?));
        }

        if self.is_func_decl(false) {
            return Ok(ScriptNode::Func(self.parse_function(false)?));
        }

        Ok(ScriptNode::Var(self.parse_var(false, true)?))
    }

    fn parse_directive(&mut self) -> Result<ScriptNode> {
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
                let name = self.current().span.source.clone();
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

    fn parse_namespace(&mut self) -> Result<Namespace> {
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

        Ok(Namespace { name, items })
    }

    fn parse_enum(&mut self) -> Result<Enum> {
        let mut modifiers = Vec::new();

        while self.identifier_is("shared") || self.identifier_is("external") {
            modifiers.push(self.expect_identifier()?);
        }

        self.expect(&TokenKind::Enum)?;
        let name = self.expect_identifier()?;

        if self.check(&TokenKind::Semicolon) {
            self.advance();
            return Ok(Enum {
                modifiers,
                name,
                variants: Vec::new(),
            });
        }

        self.expect(&TokenKind::LBrace)?;

        let mut variants = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let variant_name = self.expect_identifier()?;
            let value = if self.check(&TokenKind::Assign) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                value,
            });

            if !self.check(&TokenKind::RBrace) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Enum {
            modifiers,
            name,
            variants,
        })
    }

    fn parse_class(&mut self) -> Result<Class> {
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
            return Ok(Class {
                modifiers,
                name,
                extends,
                members: Vec::new(),
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

        Ok(Class {
            modifiers,
            name,
            extends,
            members,
        })
    }

    fn parse_class_member(&mut self, class_name: &str) -> Result<ClassMember> {
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

    fn parse_interface(&mut self) -> Result<Interface> {
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
            return Ok(Interface {
                modifiers,
                name,
                extends,
                members: Vec::new(),
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

        Ok(Interface {
            modifiers,
            name,
            extends,
            members,
        })
    }

    fn parse_interface_method(&mut self) -> Result<IntfMthd> {
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

        Ok(IntfMthd {
            return_type,
            is_ref,
            name,
            params,
            is_const,
        })
    }

    fn parse_typedef(&mut self) -> Result<Typedef> {
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

        Ok(Typedef { prim_type, name })
    }

    fn parse_funcdef(&mut self) -> Result<FuncDef> {
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

        Ok(FuncDef {
            modifiers,
            return_type,
            is_ref,
            name,
            params,
        })
    }

    fn parse_mixin(&mut self) -> Result<Mixin> {
        self.expect(&TokenKind::Mixin)?;
        let class = self.parse_class()?;
        Ok(Mixin { class })
    }

    fn parse_import(&mut self) -> Result<Import> {
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

        Ok(Import {
            type_name,
            is_ref,
            identifier,
            params,
            from,
        })
    }

    fn parse_virtprop(&mut self, is_method: bool, is_interface: bool) -> Result<VirtProp> {
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

            accessors.push(PropertyAccessor {
                kind,
                is_const,
                attributes,
                body,
            });
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(VirtProp {
            visibility,
            prop_type,
            is_ref,
            name,
            accessors,
        })
    }

    fn parse_function(&mut self, is_method: bool) -> Result<Func> {
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
        })
    }

    pub(crate) fn parse_var(&mut self, is_class_prop: bool, is_global_var: bool) -> Result<Var> {
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

            declarations.push(VarDecl { name, initializer });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.expect(&TokenKind::Semicolon)?;

        Ok(Var {
            visibility,
            var_type,
            declarations,
        })
    }

    fn superficially_parse_var_init(&mut self) -> Result<VarInit> {
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

            Ok(VarInit::Expr(Expr::Void))
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

    pub fn parse_type(&mut self) -> Result<Type> {
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

        Ok(Type {
            is_const,
            scope,
            datatype,
            template_types,
            modifiers,
        })
    }

    fn parse_scope(&mut self) -> Result<Scope> {
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

    fn parse_datatype(&mut self) -> Result<DataType> {
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

    fn parse_template_args(&mut self) -> Result<Vec<Type>> {
        self.expect(&TokenKind::Lt)?;

        let mut types = vec![self.parse_type()?];

        while self.check(&TokenKind::Comma) {
            self.advance();
            types.push(self.parse_type()?);
        }

        self.expect_gt_in_template()?;

        Ok(types)
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>> {
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

    fn parse_param(&mut self) -> Result<Param> {
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

        Ok(Param {
            param_type,
            type_mod,
            name,
            default_value,
            is_variadic: false,
        })
    }

    fn parse_func_attributes(&mut self) -> Result<Vec<String>> {
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

    fn parse_stat_block(&mut self) -> Result<StatBlock> {
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

        Ok(StatBlock { statements })
    }

    fn try_parse_type(&mut self) -> Result<Type> {
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

    fn expect_gt_in_template(&mut self) -> Result<()> {
        match &self.current().kind {
            TokenKind::Gt => {
                self.advance();
                Ok(())
            }
            TokenKind::Shr => {
                let shr_token = self.current().clone();
                let gt_token = Token::new(
                    TokenKind::Gt,
                    Span::new(
                        Position::new(
                            shr_token.span.start.line,
                            shr_token.span.start.column + 1,
                            shr_token.span.start.offset + 1,
                        ),
                        shr_token.span.end.clone(),
                        ">".to_string(),
                    ),
                );
                self.tokens[self.pos] = gt_token;
                Ok(())
            }
            TokenKind::UShr => {
                let ushr_token = self.current().clone();
                let shr_token = Token::new(
                    TokenKind::Shr,
                    Span::new(
                        Position::new(
                            ushr_token.span.start.line,
                            ushr_token.span.start.column + 1,
                            ushr_token.span.start.offset + 1,
                        ),
                        ushr_token.span.end.clone(),
                        ">>".to_string(),
                    ),
                );
                self.tokens[self.pos] = shr_token;
                Ok(())
            }
            _ => Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: "'>'".to_string(),
                found: format!("{:?}", self.current().kind),
            }),
        }
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        match &self.current().kind {
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::ForEach => self.parse_foreach(), // Direct match!
            TokenKind::While => self.parse_while(),
            TokenKind::Do => self.parse_do_while(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => {
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(Statement::Break)
            }
            TokenKind::Continue => {
                self.advance();
                self.expect(&TokenKind::Semicolon)?;
                Ok(Statement::Continue)
            }
            TokenKind::Try => self.parse_try(),
            TokenKind::LBrace => Ok(Statement::Block(self.parse_stat_block()?)),
            _ => self.parse_expr_statement(),
        }
    }

    fn parse_foreach(&mut self) -> Result<Statement> {
        // Parse: foreach( type var [, type var]* : container ) statement

        self.expect(&TokenKind::ForEach)?; // Simplified!
        self.expect(&TokenKind::LParen)?;

        // Parse variable declarations: type name [, type name]*
        let mut variables = Vec::new();

        loop {
            // Parse type
            let var_type = self.parse_type()?;

            // Parse variable name
            let var_name = self.expect_identifier()?;

            variables.push((var_type, var_name));

            // Check for comma (more variables) or colon (end of variables)
            if self.check(&TokenKind::Comma) {
                self.advance();
                // Continue to next variable
            } else if self.check(&TokenKind::Colon) {
                break;
            } else {
                return Err(ParseError::UnexpectedToken {
                    span: self.current().span.clone(),
                    expected: "',' or ':'".to_string(),
                    found: format!("{:?}", self.current().kind),
                });
            }
        }

        self.expect(&TokenKind::Colon)?;

        // Parse the iterable expression
        let iterable = self.parse_expression()?;

        self.expect(&TokenKind::RParen)?;

        // Parse the body
        let body = Box::new(self.parse_statement()?);

        Ok(Statement::ForEach(ForEachStmt {
            variables,
            iterable,
            body,
        }))
    }

    fn parse_if(&mut self) -> Result<Statement> {
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

        Ok(Statement::If(IfStmt {
            condition,
            then_branch,
            else_branch,
        }))
    }

    fn parse_for(&mut self) -> Result<Statement> {
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

        Ok(Statement::For(ForStmt {
            init,
            condition,
            increment,
            body,
        }))
    }

    fn parse_while(&mut self) -> Result<Statement> {
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;

        let body = Box::new(self.parse_statement()?);

        Ok(Statement::While(WhileStmt { condition, body }))
    }

    fn parse_do_while(&mut self) -> Result<Statement> {
        self.expect(&TokenKind::Do)?;
        let body = Box::new(self.parse_statement()?);
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;
        self.expect(&TokenKind::Semicolon)?;

        Ok(Statement::DoWhile(DoWhileStmt { body, condition }))
    }

    fn parse_switch(&mut self) -> Result<Statement> {
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

        Ok(Statement::Switch(SwitchStmt { value, cases }))
    }

    fn parse_case(&mut self) -> Result<Case> {
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

        Ok(Case {
            pattern,
            statements,
        })
    }

    fn parse_return(&mut self) -> Result<Statement> {
        self.expect(&TokenKind::Return)?;

        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(&TokenKind::Semicolon)?;

        Ok(Statement::Return(ReturnStmt { value }))
    }

    fn parse_try(&mut self) -> Result<Statement> {
        self.expect(&TokenKind::Try)?;
        let try_block = self.parse_stat_block()?;
        self.expect(&TokenKind::Catch)?;
        let catch_block = self.parse_stat_block()?;

        Ok(Statement::Try(TryStmt {
            try_block,
            catch_block,
        }))
    }

    fn parse_expr_statement(&mut self) -> Result<Statement> {
        let expr = self.parse_expr_statement_inner()?;
        Ok(Statement::Expr(expr))
    }

    fn parse_expr_statement_inner(&mut self) -> Result<Option<Expr>> {
        if self.check(&TokenKind::Semicolon) {
            self.advance();
            return Ok(None);
        }

        let expr = self.parse_expression()?;
        self.expect(&TokenKind::Semicolon)?;
        Ok(Some(expr))
    }

    fn parse_expression(&mut self) -> Result<Expr> {
        let expr_tokens = self.collect_expression_tokens()?;

        if expr_tokens.is_empty() {
            return Err(self.error("Expected expression"));
        }

        let pratt = ExprParser::new(expr_tokens);
        pratt.parse()
    }

    fn collect_expression_tokens(&mut self) -> Result<Vec<Token>> {
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
                    // Look ahead to see if this is actually a template
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
        // Check pattern: identifier < type_stuff >
        // where type_stuff is: type [, type]*

        // Must have identifier before <
        if tokens_so_far.is_empty() {
            return false;
        }
        if !matches!(tokens_so_far.last().unwrap().kind, TokenKind::Identifier(_)) {
            return false;
        }

        // Look ahead past the < (current position is at <, so start at pos + 1)
        let mut scan_pos = self.pos + 1;
        let mut depth: i32 = 1;
        let mut saw_type = false;

        while scan_pos < self.tokens.len() && depth > 0 {
            match &self.tokens[scan_pos].kind {
                // Type tokens
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

                // Template nesting
                TokenKind::Lt => {
                    depth += 1;
                    scan_pos += 1;
                }
                TokenKind::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        // Found matching >
                        // This looks like a template if we saw types
                        return saw_type;
                    }
                    scan_pos += 1;
                }
                TokenKind::Shr => {
                    // >> closes 2 levels
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
                    // >>> closes 3 levels
                    depth = depth.saturating_sub(3);
                    if depth == 0 {
                        return saw_type;
                    }
                    scan_pos += 1;
                }

                // Comma is OK in templates
                TokenKind::Comma => {
                    scan_pos += 1;
                }

                // Scope resolution is OK in types
                TokenKind::DoubleColon => {
                    scan_pos += 1;
                }

                // Array brackets and @ are OK in types
                TokenKind::LBracket | TokenKind::RBracket | TokenKind::At => {
                    scan_pos += 1;
                }

                // Operators indicate this is NOT a template
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
                | TokenKind::Assign => {
                    return false;
                }

                // Numbers indicate comparison, not template
                TokenKind::Number(_) => {
                    return false;
                }

                // Anything else, stop looking
                _ => return false,
            }

            // Safety: don't scan too far (prevent infinite loops)
            if scan_pos - self.pos > 100 {
                return false;
            }
        }

        // Didn't find matching >, not a template
        false
    }

    fn parse_init_list(&mut self) -> Result<InitList> {
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

        Ok(InitList { items })
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Arg>> {
        self.expect(&TokenKind::LParen)?;

        if self.check(&TokenKind::RParen) {
            self.advance();
            return Ok(Vec::new());
        }

        let mut args = Vec::new();

        loop {
            let value = self.parse_expression()?;
            args.push(Arg { name: None, value });

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

    fn expect(&mut self, kind: &TokenKind) -> Result<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: format!("{:?}", kind),
                found: format!("{:?}", self.current().kind),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<String> {
        match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("Expected identifier")),
        }
    }

    fn expect_string(&mut self) -> Result<String> {
        if let TokenKind::String(s) = &self.current().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(self.error("Expected string literal"))
        }
    }

    fn expect_contextual_keyword(&mut self, keyword: &str) -> Result<()> {
        if self.identifier_is(keyword) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("Expected keyword '{}'", keyword)))
        }
    }

    fn read_until_newline(&mut self) -> String {
        let mut content = String::new();
        let start_line = self.current().span.start.line;

        while !self.is_at_end() && self.current().span.start.line == start_line {
            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(&self.current().span.source);
            self.advance();
        }

        content.trim().to_string()
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError::SyntaxError {
            span: self.current().span.clone(),
            message: message.to_string(),
        }
    }

    /// Parse a function declaration without body (for engine registration)
    /// This is a public helper for the declaration parser
    pub fn parse_function_signature(&mut self) -> Result<Func> {
        let mut modifiers = Vec::new();
        let mut visibility = None;

        // Parse modifiers
        while self.identifier_is("shared")
            || self.identifier_is("external")
            || self.identifier_is("final")
            || self.identifier_is("override")
            || self.identifier_is("virtual")
            || self.identifier_is("explicit")
        {
            modifiers.push(self.expect_identifier()?);
        }

        // Parse visibility
        if self.check(&TokenKind::Private) {
            visibility = Some(Visibility::Private);
            self.advance();
        } else if self.check(&TokenKind::Protected) {
            visibility = Some(Visibility::Protected);
            self.advance();
        }

        // Parse return type (optional for constructors)
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

        // Parse function name
        let name = self.expect_identifier()?;

        // Parse parameters
        let params = self.parse_param_list()?;

        // Check for const
        let is_const = if self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        // Parse attributes
        let attributes = self.parse_func_attributes()?;

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
        })
    }
}
