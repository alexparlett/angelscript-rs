use crate::parser::ast::*;
use crate::parser::error::*;
use crate::parser::token::*;
use crate::parser::expr_parser::ExprParser;

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

    fn parse_script_item(&mut self) -> Result<ScriptItem> {
        // Handle preprocessor directives
        if self.check(&TokenKind::Hash) {
            return self.parse_directive();
        }

        // Handle modifiers
        let modifiers = self.parse_modifiers();

        match self.current().kind {
            TokenKind::Namespace => Ok(ScriptItem::Namespace(self.parse_namespace()?)),
            TokenKind::Enum => Ok(ScriptItem::Enum(self.parse_enum(modifiers)?)),
            TokenKind::Class => Ok(ScriptItem::Class(self.parse_class(modifiers)?)),
            TokenKind::Interface => Ok(ScriptItem::Interface(self.parse_interface(modifiers)?)),
            TokenKind::Typedef => Ok(ScriptItem::Typedef(self.parse_typedef()?)),
            TokenKind::Funcdef => Ok(ScriptItem::FuncDef(self.parse_funcdef(modifiers)?)),
            TokenKind::Mixin => Ok(ScriptItem::Mixin(self.parse_mixin()?)),
            TokenKind::Import => Ok(ScriptItem::Import(self.parse_import()?)),
            _ => {
                // Could be function or variable
                // Quick lookahead: if we see '(' before ';' or '=', it's a function
                let checkpoint = self.pos;
                let mut found_paren = false;
                let mut found_terminator = false;

                // Skip type and identifier to check what comes next
                while !self.is_at_end() && !found_terminator {
                    match self.current().kind {
                        TokenKind::LParen => {
                            found_paren = true;
                            break;
                        }
                        TokenKind::Semicolon | TokenKind::Assign | TokenKind::Comma => {
                            found_terminator = true;
                            break;
                        }
                        _ => self.advance(),
                    }
                }

                // Restore position
                self.pos = checkpoint;

                if found_paren {
                    // It's a function
                    let visibility = self.parse_visibility(&modifiers);
                    Ok(ScriptItem::Func(self.try_parse_func(modifiers, visibility)?))
                } else {
                    // It's a variable
                    Ok(ScriptItem::Var(self.parse_var(modifiers)?))
                }
            }
        }
    }

    fn parse_directive(&mut self) -> Result<ScriptItem> {
        self.expect(&TokenKind::Hash)?;

        // Get directive name - could be a keyword or identifier
        let directive_name = match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            // Handle directive keywords that conflict with language keywords
            TokenKind::If => {
                self.advance();
                "if".to_string()
            }
            TokenKind::Else => {
                self.advance();
                "else".to_string()
            }
            _ => {
                // Try to get any token as string for directive name
                let name = self.current().span.source.clone();
                self.advance();
                name
            }
        };

        match directive_name.as_str() {
            "include" => {
                let path = self.expect_string()?;
                Ok(ScriptItem::Include(Include { path }))
            }
            "pragma" => {
                let content = self.read_until_newline();
                Ok(ScriptItem::Pragma(Pragma { content }))
            }
            "if" | "elif" | "else" | "endif" => {
                // These need special handling for conditional compilation
                // For now, just consume the rest of the line
                let content = self.read_until_newline();
                Ok(ScriptItem::CustomDirective(CustomDirective {
                    name: directive_name,
                    content,
                }))
            }
            _ => {
                let content = self.read_until_newline();
                Ok(ScriptItem::CustomDirective(CustomDirective {
                    name: directive_name,
                    content,
                }))
            }
        }
    }

    fn parse_modifiers(&mut self) -> Vec<String> {
        let mut modifiers = Vec::new();

        while matches!(
            self.current().kind,
            TokenKind::Shared
                | TokenKind::External
                | TokenKind::Abstract
                | TokenKind::Final
                | TokenKind::Private
                | TokenKind::Protected
        ) {
            modifiers.push(self.current().span.source.clone());
            self.advance();
        }

        modifiers
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

    fn parse_enum(&mut self, modifiers: Vec<String>) -> Result<Enum> {
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

    fn parse_class(&mut self, modifiers: Vec<String>) -> Result<Class> {
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
            members.push(self.parse_class_member()?);
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Class {
            modifiers,
            name,
            extends,
            members,
        })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember> {
        let member_modifiers = self.parse_modifiers();

        // Check for destructor
        if self.check(&TokenKind::BitNot) {
            return Ok(ClassMember::Func(self.parse_destructor(member_modifiers)?));
        }

        // Try to parse type
        let checkpoint = self.pos;
        if let Ok(_typ) = self.try_parse_type() {
            // Check if it's a property (has { after identifier)
            if self.check(&TokenKind::BitAnd) {
                self.advance();
            }

            if let TokenKind::Identifier(_) = self.current().kind {
                let _name = self.expect_identifier()?;

                if self.check(&TokenKind::LBrace) {
                    // It's a property
                    self.pos = checkpoint;
                    return Ok(ClassMember::VirtProp(self.parse_virtprop(member_modifiers)?));
                }

                if self.check(&TokenKind::LParen) {
                    // It's a method
                    self.pos = checkpoint;
                    return Ok(ClassMember::Func(self.parse_method(member_modifiers)?));
                }

                // It's a field
                self.pos = checkpoint;
                return Ok(ClassMember::Var(self.parse_var(member_modifiers)?));
            }
        }

        // Could be constructor or funcdef
        self.pos = checkpoint;

        if self.check_identifier() {
            let _name = self.current().span.source.clone();
            let next_pos = self.pos + 1;

            if next_pos < self.tokens.len() && self.tokens[next_pos].kind == TokenKind::LParen {
                // Constructor
                return Ok(ClassMember::Func(self.parse_constructor(member_modifiers)?));
            }
        }

        Err(self.error("Expected class member"))
    }

    fn parse_interface(&mut self, modifiers: Vec<String>) -> Result<Interface> {
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
            members.push(self.parse_interface_member()?);
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Interface {
            modifiers,
            name,
            extends,
            members,
        })
    }

    fn parse_interface_member(&mut self) -> Result<InterfaceMember> {
        let typ = self.parse_type()?;
        let is_ref = self.check(&TokenKind::BitAnd);
        if is_ref {
            self.advance();
        }

        let name = self.expect_identifier()?;

        if self.check(&TokenKind::LBrace) {
            // Property
            Ok(InterfaceMember::VirtProp(VirtProp {
                visibility: None,
                prop_type: typ,
                is_ref,
                name,
                accessors: Vec::new(), // Parse accessors if needed
            }))
        } else {
            // Method
            let params = self.parse_param_list()?;
            let is_const = self.check(&TokenKind::Const);
            if is_const {
                self.advance();
            }

            self.expect(&TokenKind::Semicolon)?;

            Ok(InterfaceMember::Method(IntfMthd {
                return_type: typ,
                is_ref,
                name,
                params,
                is_const,
            }))
        }
    }

    fn parse_typedef(&mut self) -> Result<Typedef> {
        self.expect(&TokenKind::Typedef)?;

        let prim_type = match &self.current().kind {
            TokenKind::Void => "void",
            TokenKind::Int => "int",
            TokenKind::Int8 => "int8",
            TokenKind::Int16 => "int16",
            TokenKind::Int32 => "int32",
            TokenKind::Int64 => "int64",
            TokenKind::Uint => "uint",
            TokenKind::Uint8 => "uint8",
            TokenKind::Uint16 => "uint16",
            TokenKind::Uint32 => "uint32",
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

    fn parse_funcdef(&mut self, modifiers: Vec<String>) -> Result<FuncDef> {
        self.expect(&TokenKind::Funcdef)?;

        let return_type = self.parse_type()?;
        let is_ref = self.check(&TokenKind::BitAnd);
        if is_ref {
            self.advance();
        }

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
        let class = self.parse_class(Vec::new())?;
        Ok(Mixin { class })
    }

    fn parse_import(&mut self) -> Result<Import> {
        self.expect(&TokenKind::Import)?;

        let type_name = self.parse_type()?;
        let is_ref = self.check(&TokenKind::BitAnd);
        if is_ref {
            self.advance();
        }

        let identifier = self.expect_identifier()?;
        let params = self.parse_param_list()?;

        // Parse function attributes
        while matches!(
            self.current().kind,
            TokenKind::Override | TokenKind::Final | TokenKind::Explicit | TokenKind::Property
        ) {
            self.advance();
        }

        self.expect_keyword("from")?;
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

    fn parse_virtprop(&mut self, modifiers: Vec<String>) -> Result<VirtProp> {
        let visibility = self.parse_visibility(&modifiers);

        let prop_type = self.parse_type()?;
        let is_ref = self.check(&TokenKind::BitAnd);
        if is_ref {
            self.advance();
        }

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut accessors = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let kind = if self.check(&TokenKind::Get) {
                self.advance();
                AccessorKind::Get
            } else if self.check(&TokenKind::Set) {
                self.advance();
                AccessorKind::Set
            } else {
                return Err(self.error("Expected 'get' or 'set'"));
            };

            let is_const = self.check(&TokenKind::Const);
            if is_const {
                self.advance();
            }

            let attributes = self.parse_func_attributes();

            let body = if self.check(&TokenKind::LBrace) {
                Some(self.parse_stat_block()?)
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

    fn parse_func_or_var(&mut self, modifiers: Vec<String>) -> Result<ScriptItem> {
        let visibility = self.parse_visibility(&modifiers);
        let checkpoint = self.pos;

        // Try to parse as function
        match self.try_parse_func(modifiers.clone(), visibility) {
            Ok(func) => Ok(ScriptItem::Func(func)),
            Err(_) => {
                // Backtrack and try as variable
                self.pos = checkpoint;
                Ok(ScriptItem::Var(self.parse_var(modifiers)?))
            }
        }
    }

    fn try_parse_func(&mut self, modifiers: Vec<String>, visibility: Option<Visibility>) -> Result<Func> {
        // Check for destructor
        if self.check(&TokenKind::BitNot) {
            return self.parse_destructor(modifiers);
        }

        // Try to parse return type
        let return_type = if self.check_type_start() {
            Some(self.parse_type()?)
        } else {
            None
        };

        let is_ref = self.check(&TokenKind::BitAnd);
        if is_ref {
            self.advance();
        }

        let name = self.expect_identifier()?;

        // Must have parameter list for function
        if !self.check(&TokenKind::LParen) {
            return Err(self.error("Expected '(' for function"));
        }

        let params = self.parse_param_list()?;

        let is_const = self.check(&TokenKind::Const);
        if is_const {
            self.advance();
        }

        let attributes = self.parse_func_attributes();

        let body = if self.check(&TokenKind::LBrace) {
            Some(self.parse_stat_block()?)
        } else {
            self.expect(&TokenKind::Semicolon)?;
            None
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

    fn parse_destructor(&mut self, modifiers: Vec<String>) -> Result<Func> {
        self.expect(&TokenKind::BitNot)?;
        let name = format!("~{}", self.expect_identifier()?);
        let params = self.parse_param_list()?;
        let attributes = self.parse_func_attributes();

        let body = if self.check(&TokenKind::LBrace) {
            Some(self.parse_stat_block()?)
        } else {
            self.expect(&TokenKind::Semicolon)?;
            None
        };

        Ok(Func {
            modifiers,
            visibility: None,
            return_type: None,
            is_ref: false,
            name,
            params,
            is_const: false,
            attributes,
            body,
        })
    }

    fn parse_constructor(&mut self, modifiers: Vec<String>) -> Result<Func> {
        let name = self.expect_identifier()?;
        let params = self.parse_param_list()?;
        let attributes = self.parse_func_attributes();

        let body = if self.check(&TokenKind::LBrace) {
            Some(self.parse_stat_block()?)
        } else {
            self.expect(&TokenKind::Semicolon)?;
            None
        };

        Ok(Func {
            modifiers,
            visibility: None,
            return_type: None,
            is_ref: false,
            name,
            params,
            is_const: false,
            attributes,
            body,
        })
    }

    fn parse_method(&mut self, modifiers: Vec<String>) -> Result<Func> {
        let visibility = self.parse_visibility(&modifiers);
        let return_type = Some(self.parse_type()?);

        let is_ref = self.check(&TokenKind::BitAnd);
        if is_ref {
            self.advance();
        }

        let name = self.expect_identifier()?;
        let params = self.parse_param_list()?;

        let is_const = self.check(&TokenKind::Const);
        if is_const {
            self.advance();
        }

        let attributes = self.parse_func_attributes();

        let body = if self.check(&TokenKind::LBrace) {
            Some(self.parse_stat_block()?)
        } else {
            self.expect(&TokenKind::Semicolon)?;
            None
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

    fn parse_var(&mut self, modifiers: Vec<String>) -> Result<Var> {
        let visibility = self.parse_visibility(&modifiers);
        let var_type = self.parse_type()?;

        let mut declarations = Vec::new();

        loop {
            let name = self.expect_identifier()?;

            let initializer = if self.check(&TokenKind::Assign) {
                self.advance();

                // Check for init list
                if self.check(&TokenKind::LBrace) {
                    Some(VarInit::InitList(self.parse_init_list()?))
                } else {
                    // Parse as expression (handles lambdas, function calls, etc.)
                    Some(VarInit::Expr(self.parse_expression()?))
                }
            } else if self.check(&TokenKind::LParen) {
                Some(VarInit::ArgList(self.parse_arg_list()?))
            } else {
                None
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

    fn parse_type(&mut self) -> Result<Type> {
        let is_const = self.check(&TokenKind::Const);
        if is_const {
            self.advance();
        }

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

    fn try_parse_type(&mut self) -> Result<Type> {
        self.parse_type()
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
                // Not part of scope, backtrack
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
            TokenKind::Int => {
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
            TokenKind::Int32 => {
                self.advance();
                Ok(DataType::PrimType("int32".to_string()))
            }
            TokenKind::Int64 => {
                self.advance();
                Ok(DataType::PrimType("int64".to_string()))
            }
            TokenKind::Uint => {
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
            TokenKind::Uint32 => {
                self.advance();
                Ok(DataType::PrimType("uint32".to_string()))
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

        self.expect(&TokenKind::Gt)?;

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
                // Default is inout
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

    fn parse_func_attributes(&mut self) -> Vec<String> {
        let mut attributes = Vec::new();

        while matches!(
            self.current().kind,
            TokenKind::Override | TokenKind::Final | TokenKind::Explicit | TokenKind::Property
        ) {
            attributes.push(self.current().span.source.clone());
            self.advance();
        }

        attributes
    }

    fn parse_stat_block(&mut self) -> Result<StatBlock> {
        self.expect(&TokenKind::LBrace)?;

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            // Check for variable declaration
            if self.is_var_declaration() {
                statements.push(Statement::Var(self.parse_var(Vec::new())?));
            } else {
                statements.push(self.parse_statement()?);
            }
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(StatBlock { statements })
    }

    fn is_var_declaration(&mut self) -> bool {
        let checkpoint = self.pos;

        // Check for visibility modifiers
        if matches!(self.current().kind, TokenKind::Private | TokenKind::Protected) {
            self.advance();
        }

        // Check for const
        if self.check(&TokenKind::Const) {
            self.advance();
        }

        // Must start with a type keyword or identifier
        let looks_like_type = matches!(
        self.current().kind,
        TokenKind::Void | TokenKind::Int | TokenKind::Int8 | TokenKind::Int16 |
        TokenKind::Int32 | TokenKind::Int64 | TokenKind::Uint | TokenKind::Uint8 |
        TokenKind::Uint16 | TokenKind::Uint32 | TokenKind::Uint64 | TokenKind::Float |
        TokenKind::Double | TokenKind::Bool | TokenKind::Auto | TokenKind::Identifier(_)
    );

        if !looks_like_type {
            self.pos = checkpoint;
            return false;
        }

        // Try to parse type
        let result = if self.try_parse_type().is_ok() {
            // After type, should be identifier (variable name)
            self.check_identifier()
        } else {
            false
        };

        self.pos = checkpoint;
        result
    }

    fn check_type_start(&self) -> bool {
        matches!(
            self.current().kind,
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
                | TokenKind::Identifier(_)
                | TokenKind::DoubleColon
        )
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        match &self.current().kind {
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::Foreach => self.parse_foreach(),
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

        let init = if self.is_var_declaration() {
            ForInit::Var(self.parse_var(Vec::new())?)
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

    fn parse_foreach(&mut self) -> Result<Statement> {
        self.expect(&TokenKind::Foreach)?;
        self.expect(&TokenKind::LParen)?;

        let mut variables = Vec::new();

        loop {
            let var_type = self.parse_type()?;
            let var_name = self.expect_identifier()?;
            variables.push((var_type, var_name));

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.expect(&TokenKind::Colon)?;
        let iterable = self.parse_expression()?;
        self.expect(&TokenKind::RParen)?;

        let body = Box::new(self.parse_statement()?);

        Ok(Statement::ForEach(ForEachStmt {
            variables,
            iterable,
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
            if self.is_var_declaration() {
                statements.push(Statement::Var(self.parse_var(Vec::new())?));
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
        // Collect tokens until we hit a statement terminator
        let expr_tokens = self.collect_expression_tokens()?;

        if expr_tokens.is_empty() {
            return Err(self.error("Expected expression"));
        }

        // Use Pratt parser
        let pratt = ExprParser::new(expr_tokens);
        pratt.parse()
    }

    fn collect_expression_tokens(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut paren_depth = 0;
        let mut bracket_depth = 0;
        let mut brace_depth = 0;

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
                    // LBrace starts init list
                    brace_depth += 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::RBrace => {
                    if brace_depth == 0 {
                        // Not in an init list, end expression
                        break;
                    }
                    brace_depth -= 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Semicolon => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        break;
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Comma => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        break;
                    }
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::Colon => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        // Check if it's part of ternary
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
            // Check for named argument
            let name = if self.check_identifier() {
                let checkpoint = self.pos;
                let ident = self.expect_identifier()?;

                if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(ident)
                } else {
                    self.pos = checkpoint;
                    None
                }
            } else {
                None
            };

            let value = self.parse_expression()?;
            args.push(Arg { name, value });

            if !self.check(&TokenKind::Comma) {
                break;
            }
            self.advance();
        }

        self.expect(&TokenKind::RParen)?;

        Ok(args)
    }

    // Helper methods

    fn parse_visibility(&self, modifiers: &[String]) -> Option<Visibility> {
        if modifiers.contains(&"private".to_string()) {
            Some(Visibility::Private)
        } else if modifiers.contains(&"protected".to_string()) {
            Some(Visibility::Protected)
        } else {
            None
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
            // Allow some keywords to be used as identifiers in certain contexts
            TokenKind::Function => {
                self.advance();
                Ok("function".to_string())
            }
            _ => Err(self.error("Expected identifier"))
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

    fn expect_keyword(&mut self, keyword: &str) -> Result<()> {
        if let TokenKind::Identifier(name) = &self.current().kind {
            if name == keyword {
                self.advance();
                return Ok(());
            }
        }
        Err(self.error(&format!("Expected keyword '{}'", keyword)))
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
}
