//! Declaration parsing functions for AngelScript.
//!
//! Implements parsing of all top-level declarations including functions,
//! classes, interfaces, enums, and namespaces.

use crate::ast::{DeclModifiers, FuncAttr, Ident, ParseError, ParseErrorKind, PropertyAccessorKind as PropAccessorKind, Visibility};
use crate::ast::decl::*;
use crate::ast::types::ParamType;
use crate::lexer::{Span, TokenKind};
use super::parser::Parser;

impl<'src> Parser<'src> {
    /// Parse a complete script.
    ///
    /// This is the main entry point for parsing AngelScript source code.
    pub fn parse_script(&mut self) -> Result<Script, ParseError> {
        let start_span = self.peek().span;
        let mut items = Vec::new();

        // Parse all top-level items
        while !self.is_eof() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(err) => {
                    // Record the error and try to recover
                    self.errors.push(err);
                    self.synchronize();
                    if self.is_eof() {
                        break;
                    }
                }
            }
        }

        let end_span = if let Some(last_item) = items.last() {
            last_item.span()
        } else {
            start_span
        };

        Ok(Script {
            items,
            span: start_span.merge(end_span),
        })
    }

    /// Parse a top-level item.
    pub fn parse_item(&mut self) -> Result<Item, ParseError> {
        // Skip empty statements
        if self.eat(TokenKind::Semicolon).is_some() {
            return self.parse_item();
        }

        // Parse modifiers
        let modifiers = self.parse_modifiers()?;
        let visibility = self.parse_visibility()?;

        let token = self.peek().clone();

        match token.kind {
            TokenKind::Class => self.parse_class(modifiers, visibility),
            TokenKind::Interface => self.parse_interface(modifiers),
            TokenKind::Enum => self.parse_enum(modifiers),
            TokenKind::FuncDef => self.parse_funcdef(modifiers),
            TokenKind::Namespace => self.parse_namespace(),
            TokenKind::Typedef => self.parse_typedef(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Mixin => self.parse_mixin(),
            
            // Function or global variable
            _ => {
                // Try to parse as function or global variable
                self.parse_function_or_global_var(modifiers, visibility)
            }
        }
    }

    /// Parse declaration modifiers (shared, external, abstract, final).
    fn parse_modifiers(&mut self) -> Result<DeclModifiers, ParseError> {
        let mut modifiers = DeclModifiers::new();

        loop {
            if self.check_contextual("shared") {
                if modifiers.shared {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'shared' modifier",
                    );
                }
                self.advance();
                modifiers.shared = true;
            } else if self.check_contextual("external") {
                if modifiers.external {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'external' modifier",
                    );
                }
                self.advance();
                modifiers.external = true;
            } else if self.check_contextual("abstract") {
                if modifiers.abstract_ {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'abstract' modifier",
                    );
                }
                self.advance();
                modifiers.abstract_ = true;
            } else if self.check_contextual("final") {
                if modifiers.final_ {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'final' modifier",
                    );
                }
                self.advance();
                modifiers.final_ = true;
            } else {
                break;
            }
        }

        Ok(modifiers)
    }

    /// Parse visibility modifier (private, protected, or default to public).
    fn parse_visibility(&mut self) -> Result<Visibility, ParseError> {
        if self.eat(TokenKind::Private).is_some() {
            Ok(Visibility::Private)
        } else if self.eat(TokenKind::Protected).is_some() {
            Ok(Visibility::Protected)
        } else {
            Ok(Visibility::Public)
        }
    }

    /// Parse function attributes (override, final, explicit, property, delete).
    fn parse_func_attrs(&mut self) -> Result<FuncAttr, ParseError> {
        let mut attrs = FuncAttr::new();

        loop {
            if self.check_contextual("override") {
                if attrs.override_ {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'override' attribute",
                    );
                }
                self.advance();
                attrs.override_ = true;
            } else if self.check_contextual("final") {
                if attrs.final_ {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'final' attribute",
                    );
                }
                self.advance();
                attrs.final_ = true;
            } else if self.check_contextual("explicit") {
                if attrs.explicit {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'explicit' attribute",
                    );
                }
                self.advance();
                attrs.explicit = true;
            } else if self.check_contextual("property") {
                if attrs.property {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'property' attribute",
                    );
                }
                self.advance();
                attrs.property = true;
            } else if self.check_contextual("delete") {
                if attrs.delete {
                    let span = self.peek().span;
                    self.error(
                        ParseErrorKind::ConflictingModifiers,
                        span,
                        "duplicate 'delete' attribute",
                    );
                }
                self.advance();
                attrs.delete = true;
            } else {
                break;
            }
        }

        Ok(attrs)
    }

    /// Parse function parameters.
    ///
    /// Grammar: `'(' ('void' | (TYPE TYPEMOD ('...' | IDENTIFIER? ('=' EXPR)?) (',' TYPE TYPEMOD ('...' | IDENTIFIER? ('=' EXPR)?))*))? ')'`
    pub fn parse_function_params(&mut self) -> Result<Vec<FunctionParam>, ParseError> {
        self.expect(TokenKind::LeftParen)?;

        let mut params = Vec::new();

        // Check for void or empty parameter list
        if self.check(TokenKind::RightParen) {
            self.advance();
            return Ok(params);
        }

        // Check for explicit void
        if self.eat(TokenKind::Void).is_some() {
            self.expect(TokenKind::RightParen)?;
            return Ok(params);
        }

        // Parse parameters
        params.push(self.parse_function_param()?);

        while self.eat(TokenKind::Comma).is_some() {
            params.push(self.parse_function_param()?);
        }

        self.expect(TokenKind::RightParen)?;
        Ok(params)
    }

    /// Parse a single function parameter.
    fn parse_function_param(&mut self) -> Result<FunctionParam, ParseError> {
        let start_span = self.peek().span;

        // Check for variadic parameter (...)
        // Used for application-registered variadic functions
        // Scripts cannot define variadic functions, but parser accepts them
        if self.check(TokenKind::Dot) 
            && self.peek_nth(1).kind == TokenKind::Dot 
            && self.peek_nth(2).kind == TokenKind::Dot 
        {
            self.advance(); // consume first .
            self.advance(); // consume second .
            self.advance(); // consume third .
            
            let span = start_span.merge(
                self.buffer.get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span)
            );
            
            // Variadic parameter has a dummy void type
            let void_ty = ParamType::new(
                crate::ast::types::TypeExpr::primitive(
                    crate::ast::types::PrimitiveType::Void,
                    start_span
                ),
                crate::ast::RefKind::None,
                start_span
            );
            
            return Ok(FunctionParam {
                ty: void_ty,
                name: None,
                default: None,
                is_variadic: true,
                span,
            });
        }

        // Parse parameter type
        let ty = self.parse_param_type()?;

        // Optional parameter name
        let name = if self.check(TokenKind::Identifier) {
            let token = self.advance();
            Some(Ident::new(token.lexeme, token.span))
        } else {
            None
        };

        // Optional default value
        let default = if self.eat(TokenKind::Equal).is_some() {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        let span = if let Some(ref default_expr) = default {
            start_span.merge(default_expr.span())
        } else if let Some(ref n) = name {
            start_span.merge(n.span)
        } else {
            ty.span
        };

        Ok(FunctionParam {
            ty,
            name,
            default,
            is_variadic: false,
            span,
        })
    }

    /// Parse a function or global variable declaration.
    ///
    /// This disambiguates between functions and global variables.
    fn parse_function_or_global_var(
        &mut self,
        modifiers: DeclModifiers,
        visibility: Visibility,
    ) -> Result<Item, ParseError> {
        let start_span = self.peek().span;

        // Check for destructor (~ClassName)
        let is_destructor = if self.eat(TokenKind::Tilde).is_some() {
            true
        } else {
            false
        };

        // Parse return type (or type for variable)
        let return_type = if is_destructor {
            None
        } else {
            Some(self.parse_return_type()?)
        };

        // Parse name
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        // Optional template parameters (for application-registered template functions)
        // Example: void swap<T>(T &inout a, T &inout b) { }
        // Note: Scripts cannot define template functions, but parser accepts them
        let template_params = if self.check(TokenKind::Less) {
            self.parse_template_param_names()?
        } else {
            Vec::new()
        };

        // Disambiguate: function has '(', variable has '=' or ';'
        if self.check(TokenKind::LeftParen) {
            // It's a function
            let params = self.parse_function_params()?;

            // Check for const method
            let is_const = self.eat(TokenKind::Const).is_some();

            // Parse function attributes
            let attrs = self.parse_func_attrs()?;

            // Parse body or semicolon
            let body = if self.check(TokenKind::LeftBrace) {
                Some(self.parse_block()?)
            } else {
                self.expect(TokenKind::Semicolon)?;
                None
            };

            let span = start_span.merge(
                self.buffer.get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span)
            );

            Ok(Item::Function(FunctionDecl {
                modifiers,
                visibility,
                return_type,
                name,
                template_params,
                params,
                is_const,
                attrs,
                body,
                is_destructor,
                span,
            }))
        } else {
            // It's a global variable
            if return_type.is_none() {
                return Err(ParseError::new(

                    ParseErrorKind::ExpectedType,

                    name.span,

                    "destructor syntax not valid for global variable",

                ));
            }

            let ty = return_type.unwrap().ty;

            // Optional initializer
            let init = if self.eat(TokenKind::Equal).is_some() {
                Some(self.parse_expr(0)?)
            } else {
                None
            };

            let end_span = self.expect(TokenKind::Semicolon)?.span;

            Ok(Item::GlobalVar(GlobalVarDecl {
                visibility,
                ty,
                name,
                init,
                span: start_span.merge(end_span),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Parser;

    #[test]
    fn parse_simple_function() {
        let mut parser = Parser::new("void foo() { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.name.name, "foo");
                assert!(func.body.is_some());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_with_params() {
        let mut parser = Parser::new("int add(int a, int b) { return a + b; }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 2);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_with_default_param() {
        let mut parser = Parser::new("void foo(int x = 42) { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 1);
                assert!(func.params[0].default.is_some());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_const_method() {
        let mut parser = Parser::new("int getValue() const { return 42; }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(func.is_const);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_global_var() {
        let mut parser = Parser::new("int globalCounter = 0;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::GlobalVar(var) => {
                assert_eq!(var.name.name, "globalCounter");
                assert!(var.init.is_some());
            }
            _ => panic!("Expected global variable"),
        }
    }

    #[test]
    fn parse_destructor() {
        let mut parser = Parser::new("~MyClass() { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(func.is_destructor);
                assert!(func.return_type.is_none());
            }
            _ => panic!("Expected destructor"),
        }
    }

    // ========================================================================
    // Modifier and Visibility Tests
    // ========================================================================

    #[test]
    fn parse_shared_modifier() {
        let mut parser = Parser::new("shared class Foo { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert!(class.modifiers.shared);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_external_modifier() {
        let mut parser = Parser::new("external void foo();");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(func.modifiers.external);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_abstract_modifier() {
        let mut parser = Parser::new("abstract class Base { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert!(class.modifiers.abstract_);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_final_modifier() {
        let mut parser = Parser::new("final class Sealed { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert!(class.modifiers.final_);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_multiple_modifiers() {
        let mut parser = Parser::new("shared final class Foo { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert!(class.modifiers.shared);
                assert!(class.modifiers.final_);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_private_visibility() {
        let mut parser = Parser::new("private int x = 0;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::GlobalVar(var) => {
                assert!(matches!(var.visibility, Visibility::Private));
            }
            _ => panic!("Expected global var"),
        }
    }

    #[test]
    fn parse_protected_visibility() {
        let mut parser = Parser::new("protected void foo() { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(matches!(func.visibility, Visibility::Protected));
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_default_public_visibility() {
        let mut parser = Parser::new("void foo() { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(matches!(func.visibility, Visibility::Public));
            }
            _ => panic!("Expected function"),
        }
    }

    // ========================================================================
    // Function Attribute Tests
    // ========================================================================

    #[test]
    fn parse_override_attribute() {
        let mut parser = Parser::new("void foo() override { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(func.attrs.override_);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_final_attribute() {
        let mut parser = Parser::new("void foo() final { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(func.attrs.final_);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_explicit_attribute() {
        let mut parser = Parser::new("MyClass(int x) explicit { }");
        let item = parser.parse_class_member().unwrap();
        match item {
            ClassMember::Method(func) => {
                assert!(func.attrs.explicit);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_property_attribute() {
        let mut parser = Parser::new("void setValue(int v) property { }");
        let item = parser.parse_class_member().unwrap();
        match item {
            ClassMember::Method(func) => {
                assert!(func.attrs.property);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_delete_attribute() {
        let mut parser = Parser::new("class MyClass { MyClass(const MyClass& in) delete; }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert!(class.members.len() == 1);
                let deleted = class.members.get(0);
                match deleted {
                    Some(ClassMember::Method(method)) => {
                        assert!(method.attrs.delete);
                        assert!(method.body.is_none());
                    }
                    _ => panic!("Expected method")
                }
            }
            _ => panic!("Expected class"),
        }
    }

    // ========================================================================
    // Function Parameter Tests
    // ========================================================================

    #[test]
    fn parse_function_void_params() {
        let mut parser = Parser::new("void foo(void) { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 0);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_no_params() {
        let mut parser = Parser::new("void foo() { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 0);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_unnamed_param() {
        let mut parser = Parser::new("void foo(int) { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 1);
                assert!(func.params[0].name.is_none());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_variadic() {
        let mut parser = Parser::new("void printf(const string& in fmt, ...) { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 2);
                assert!(func.params[1].is_variadic);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_template_params() {
        let mut parser = Parser::new("void swap<T>(T& inout a, T& inout b) { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert_eq!(func.template_params.len(), 1);
                assert_eq!(func.template_params[0].name, "T");
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn parse_function_declaration_only() {
        let mut parser = Parser::new("void foo();");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Function(func) => {
                assert!(func.body.is_none());
            }
            _ => panic!("Expected function"),
        }
    }

    // ========================================================================
    // Class Tests
    // ========================================================================

    #[test]
    fn parse_class_forward_declaration() {
        let mut parser = Parser::new("class Foo;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.name.name, "Foo");
                assert_eq!(class.members.len(), 0);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_inheritance() {
        let mut parser = Parser::new("class Derived : Base { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.inheritance.len(), 1);
                assert_eq!(class.inheritance[0].name, "Base");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_multiple_inheritance() {
        let mut parser = Parser::new("class Multi : Base1, Base2, Base3 { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.inheritance.len(), 3);
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_template() {
        let mut parser = Parser::new("class Container<T> { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.template_params.len(), 1);
                assert_eq!(class.template_params[0].name, "T");
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_constructor() {
        let mut parser = Parser::new(r#"
            class Point {
                Point(int x, int y) { }
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Method(method) => {
                        assert!(method.return_type.is_none());
                    }
                    _ => panic!("Expected method"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_destructor() {
        let mut parser = Parser::new(r#"
            class Foo {
                ~Foo() { }
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Method(method) => {
                        assert!(method.is_destructor);
                    }
                    _ => panic!("Expected method"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_field() {
        let mut parser = Parser::new(r#"
            class Data {
                int value;
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Field(_) => {}
                    _ => panic!("Expected field"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_field_initializer() {
        let mut parser = Parser::new(r#"
            class Data {
                int value = 42;
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                match &class.members[0] {
                    ClassMember::Field(field) => {
                        assert!(field.init.is_some());
                    }
                    _ => panic!("Expected field"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_method() {
        let mut parser = Parser::new(r#"
            class Calculator {
                int add(int a, int b) { return a + b; }
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Method(method) => {
                        assert!(method.return_type.is_some());
                        assert_eq!(method.params.len(), 2);
                    }
                    _ => panic!("Expected method"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_const_method() {
        let mut parser = Parser::new(r#"
            class Foo {
                int getValue() const { return 0; }
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                match &class.members[0] {
                    ClassMember::Method(method) => {
                        assert!(method.is_const);
                    }
                    _ => panic!("Expected method"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_virtual_property() {
        let mut parser = Parser::new(r#"
            class Foo {
                int Value {
                    get const { return 0; }
                    set { }
                }
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::VirtualProperty(prop) => {
                        assert_eq!(prop.accessors.len(), 2);
                    }
                    _ => panic!("Expected virtual property"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_with_funcdef() {
        let mut parser = Parser::new(r#"
            class Foo {
                funcdef void Callback(int x);
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Funcdef(_) => {}
                    _ => panic!("Expected funcdef"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    #[test]
    fn parse_class_member_visibility() {
        let mut parser = Parser::new(r#"
            class Foo {
                private int privateField;
                protected int protectedField;
                int publicField;
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Class(class) => {
                assert_eq!(class.members.len(), 3);
                match &class.members[0] {
                    ClassMember::Field(f) => assert!(matches!(f.visibility, Visibility::Private)),
                    _ => panic!("Expected field"),
                }
                match &class.members[1] {
                    ClassMember::Field(f) => assert!(matches!(f.visibility, Visibility::Protected)),
                    _ => panic!("Expected field"),
                }
                match &class.members[2] {
                    ClassMember::Field(f) => assert!(matches!(f.visibility, Visibility::Public)),
                    _ => panic!("Expected field"),
                }
            }
            _ => panic!("Expected class"),
        }
    }

    // ========================================================================
    // Interface Tests
    // ========================================================================

    #[test]
    fn parse_interface_simple() {
        let mut parser = Parser::new("interface IFoo { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Interface(iface) => {
                assert_eq!(iface.name.name, "IFoo");
                assert_eq!(iface.members.len(), 0);
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn parse_interface_forward_declaration() {
        let mut parser = Parser::new("interface IFoo;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Interface(iface) => {
                assert_eq!(iface.name.name, "IFoo");
                assert_eq!(iface.members.len(), 0);
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn parse_interface_with_base() {
        let mut parser = Parser::new("interface IDerived : IBase { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Interface(iface) => {
                assert_eq!(iface.bases.len(), 1);
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn parse_interface_with_method() {
        let mut parser = Parser::new(r#"
            interface IFoo {
                void doSomething();
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Interface(iface) => {
                assert_eq!(iface.members.len(), 1);
                match &iface.members[0] {
                    InterfaceMember::Method(_) => {}
                    _ => panic!("Expected method"),
                }
            }
            _ => panic!("Expected interface"),
        }
    }

    #[test]
    fn parse_interface_with_virtual_property() {
        let mut parser = Parser::new(r#"
            interface IFoo {
                int Value { get const; set; }
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Interface(iface) => {
                assert_eq!(iface.members.len(), 1);
                match &iface.members[0] {
                    InterfaceMember::VirtualProperty(_) => {}
                    _ => panic!("Expected virtual property"),
                }
            }
            _ => panic!("Expected interface"),
        }
    }

    // ========================================================================
    // Enum Tests
    // ========================================================================

    #[test]
    fn parse_enum_simple() {
        let mut parser = Parser::new("enum Color { Red, Green, Blue }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Enum(e) => {
                assert_eq!(e.name.name, "Color");
                assert_eq!(e.enumerators.len(), 3);
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn parse_enum_with_values() {
        let mut parser = Parser::new("enum Value { A = 1, B = 2, C = 4 }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Enum(e) => {
                assert_eq!(e.enumerators.len(), 3);
                assert!(e.enumerators[0].value.is_some());
                assert!(e.enumerators[1].value.is_some());
                assert!(e.enumerators[2].value.is_some());
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn parse_enum_trailing_comma() {
        let mut parser = Parser::new("enum Foo { A, B, C, }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Enum(e) => {
                assert_eq!(e.enumerators.len(), 3);
            }
            _ => panic!("Expected enum"),
        }
    }

    #[test]
    fn parse_enum_forward_declaration() {
        let mut parser = Parser::new("enum Foo;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Enum(e) => {
                assert_eq!(e.name.name, "Foo");
                assert_eq!(e.enumerators.len(), 0);
            }
            _ => panic!("Expected enum"),
        }
    }

    // ========================================================================
    // Namespace Tests
    // ========================================================================

    #[test]
    fn parse_namespace_simple() {
        let mut parser = Parser::new("namespace Foo { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Namespace(ns) => {
                assert_eq!(ns.path.len(), 1);
                assert_eq!(ns.path[0].name, "Foo");
                assert_eq!(ns.items.len(), 0);
            }
            _ => panic!("Expected namespace"),
        }
    }

    #[test]
    fn parse_namespace_nested_path() {
        let mut parser = Parser::new("namespace A::B::C { }");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Namespace(ns) => {
                assert_eq!(ns.path.len(), 3);
            }
            _ => panic!("Expected namespace"),
        }
    }

    #[test]
    fn parse_namespace_with_contents() {
        let mut parser = Parser::new(r#"
            namespace Foo {
                void bar() { }
                int x = 0;
            }
        "#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Namespace(ns) => {
                assert_eq!(ns.items.len(), 2);
            }
            _ => panic!("Expected namespace"),
        }
    }

    // ========================================================================
    // Typedef Tests
    // ========================================================================

    #[test]
    fn parse_typedef_simple() {
        let mut parser = Parser::new("typedef int MyInt;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Typedef(td) => {
                assert_eq!(td.name.name, "MyInt");
            }
            _ => panic!("Expected typedef"),
        }
    }

    #[test]
    fn parse_typedef_complex() {
        let mut parser = Parser::new("typedef array<int>@ IntArrayHandle;");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Typedef(td) => {
                assert_eq!(td.name.name, "IntArrayHandle");
            }
            _ => panic!("Expected typedef"),
        }
    }

    // ========================================================================
    // Funcdef Tests
    // ========================================================================

    #[test]
    fn parse_funcdef_simple() {
        let mut parser = Parser::new("funcdef void Callback();");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Funcdef(fd) => {
                assert_eq!(fd.name.name, "Callback");
                assert_eq!(fd.params.len(), 0);
            }
            _ => panic!("Expected funcdef"),
        }
    }

    #[test]
    fn parse_funcdef_with_params() {
        let mut parser = Parser::new("funcdef void EventHandler(int eventId, string data);");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Funcdef(fd) => {
                assert_eq!(fd.params.len(), 2);
            }
            _ => panic!("Expected funcdef"),
        }
    }

    #[test]
    fn parse_funcdef_template() {
        let mut parser = Parser::new("funcdef void Handler<T>(T value);");
        let item = parser.parse_item().unwrap();
        match item {
            Item::Funcdef(fd) => {
                assert_eq!(fd.template_params.len(), 1);
            }
            _ => panic!("Expected funcdef"),
        }
    }

    // ========================================================================
    // Import Tests
    // ========================================================================

    #[test]
    fn parse_import_simple() {
        let mut parser = Parser::new(r#"import void foo() from "module";"#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Import(imp) => {
                assert_eq!(imp.name.name, "foo");
                assert_eq!(imp.module, "module");
            }
            _ => panic!("Expected import"),
        }
    }

    #[test]
    fn parse_import_with_params() {
        let mut parser = Parser::new(r#"import int add(int a, int b) from "math";"#);
        let item = parser.parse_item().unwrap();
        match item {
            Item::Import(imp) => {
                assert_eq!(imp.params.len(), 2);
            }
            _ => panic!("Expected import"),
        }
    }

    // ========================================================================
    // Mixin Tests
    // ========================================================================

    #[test]
    fn parse_mixin() {
        let mut parser = Parser::new("mixin class Foo { }");
        let item = parser.parse_item();
        match item {
            Ok(Item::Mixin(mix)) => {
                assert_eq!(mix.class.name.name, "Foo");
            }
            _ => panic!("Expected mixin"),
        }
    }

    // ========================================================================
    // Script Parsing Tests
    // ========================================================================

    #[test]
    fn parse_script_empty() {
        let mut parser = Parser::new("");
        let script = parser.parse_script().unwrap();
        assert_eq!(script.items.len(), 0);
    }

    #[test]
    fn parse_script_multiple_items() {
        let mut parser = Parser::new(r#"
            void foo() { }
            int x = 0;
            class Bar { }
        "#);
        let script = parser.parse_script().unwrap();
        assert_eq!(script.items.len(), 3);
    }

    #[test]
    fn parse_script_with_semicolons() {
        let mut parser = Parser::new(r#"
            ;;
            void foo() { }
            ;
            int x = 0;
            ;;
        "#);
        let script = parser.parse_script().unwrap();
        assert_eq!(script.items.len(), 2);
    }
}

impl<'src> Parser<'src> {
    /// Parse a class declaration.
    ///
    /// Grammar: `'class' IDENTIFIER ((':' IDENTIFIER (',' IDENTIFIER)*)? '{' (VIRTPROP | FUNC | VAR | FUNCDEF)* '}')?`
    pub fn parse_class(
        &mut self,
        modifiers: DeclModifiers,
        _visibility: Visibility,
    ) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::Class)?.span;

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        // Optional template parameters (for application-registered template classes)
        // Example: class Container<T> { }
        // Note: Scripts cannot define template classes, but parser accepts them
        // Semantic analyzer will reject template class definitions in scripts
        let template_params = if self.check(TokenKind::Less) {
            self.parse_template_param_names()?
        } else {
            Vec::new()
        };

        // Optional inheritance
        let inheritance = if self.eat(TokenKind::Colon).is_some() {
            let mut bases = Vec::new();
            bases.push(self.parse_ident()?);

            while self.eat(TokenKind::Comma).is_some() {
                bases.push(self.parse_ident()?);
            }
            bases
        } else {
            Vec::new()
        };

        // Check for forward declaration (just semicolon)
        if self.eat(TokenKind::Semicolon).is_some() {
            let span = start_span.merge(
                self.buffer.get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span)
            );
            return Ok(Item::Class(ClassDecl {
                modifiers,
                name,
                template_params: template_params.clone(),
                inheritance,
                members: Vec::new(),
                span,
            }));
        }

        self.expect(TokenKind::LeftBrace)?;

        // Parse members
        let mut members = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_eof() {
            match self.parse_class_member() {
                Ok(member) => members.push(member),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                    if self.check(TokenKind::RightBrace) || self.is_eof() {
                        break;
                    }
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(Item::Class(ClassDecl {
            modifiers,
            name,
            template_params,
            inheritance,
            members,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a class member.
    fn parse_class_member(&mut self) -> Result<ClassMember, ParseError> {
        // Parse visibility and modifiers
        let visibility = self.parse_visibility()?;
        let modifiers = self.parse_modifiers()?;

        // Check for funcdef
        if self.check(TokenKind::FuncDef) {
            let funcdef = self.parse_funcdef(modifiers)?;
            if let Item::Funcdef(fd) = funcdef {
                return Ok(ClassMember::Funcdef(fd));
            } else {
                unreachable!()
            }
        }

        // Try to determine if this is a virtual property, method, or field
        // Constructor: IDENTIFIER '('  (no return type)
        // Virtual property: TYPE '&'? IDENTIFIER '{'
        // Method: TYPE '&'? IDENTIFIER '('
        // Field: TYPE IDENTIFIER ('=' | ';' | ',')

        // Parse type
        let ty_start = self.peek().span;
        let is_destructor = self.eat(TokenKind::Tilde).is_some();

        // Check for constructor: IDENTIFIER '(' pattern (no return type)
        // We need to distinguish between:
        // - `MyClass() { }` (constructor)
        // - `int foo() { }` (method with return type)
        let is_constructor = !is_destructor 
            && self.check(TokenKind::Identifier) 
            && self.peek_nth(1).kind == TokenKind::LeftParen;

        let (return_type, name) = if is_destructor {
            // Destructor: ~ClassName()
            let name_token = self.expect(TokenKind::Identifier)?;
            (None, Ident::new(name_token.lexeme, name_token.span))
        } else if is_constructor {
            // Constructor: ClassName()
            let name_token = self.advance();
            (None, Ident::new(name_token.lexeme, name_token.span))
        } else {
            // Regular method/field/property: TYPE name
            let return_type = Some(self.parse_return_type()?);
            let name_token = self.expect(TokenKind::Identifier)?;
            (return_type, Ident::new(name_token.lexeme, name_token.span))
        };

        // Optional template parameters (for application-registered template methods)
        // Example: template<K> void associate(K key, T value) { }
        // Methods can have their own template params in addition to class's template params
        let template_params = if self.check(TokenKind::Less) {
            self.parse_template_param_names()?
        } else {
            Vec::new()
        };

        if self.check(TokenKind::LeftBrace) {
            // Virtual property
            if return_type.is_none() {
                return Err(ParseError::new(

                    ParseErrorKind::InvalidSyntax,

                    name.span,

                    "virtual property cannot have destructor syntax",

                ));
            }

            self.advance();
            let mut accessors = Vec::new();

            while !self.check(TokenKind::RightBrace) && !self.is_eof() {
                accessors.push(self.parse_property_accessor()?);
            }

            let end_span = self.expect(TokenKind::RightBrace)?.span;

            Ok(ClassMember::VirtualProperty(VirtualPropertyDecl {
                visibility,
                ty: return_type.unwrap(),
                name,
                accessors,
                span: ty_start.merge(end_span),
            }))
        } else if self.check(TokenKind::LeftParen) {
            // Method
            let params = self.parse_function_params()?;
            let is_const = self.eat(TokenKind::Const).is_some();
            let attrs = self.parse_func_attrs()?;

            let body = if self.check(TokenKind::LeftBrace) {
                Some(self.parse_block()?)
            } else {
                self.expect(TokenKind::Semicolon)?;
                None
            };

            let span = ty_start.merge(
                self.buffer.get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(name.span)
            );

            Ok(ClassMember::Method(FunctionDecl {
                modifiers,
                visibility,
                return_type,
                name,
                template_params,  // ✅ Use method's own template params
                params,
                is_const,
                attrs,
                body,
                is_destructor,
                span,
            }))
        } else {
            // Field
            if return_type.is_none() {
                return Err(ParseError::new(

                    ParseErrorKind::InvalidSyntax,

                    name.span,

                    "field cannot have destructor syntax",

                ));
            }

            let ty = return_type.unwrap().ty;

            let init = if self.eat(TokenKind::Equal).is_some() {
                Some(self.parse_expr(0)?)
            } else {
                None
            };

            let end_span = self.expect(TokenKind::Semicolon)?.span;

            Ok(ClassMember::Field(FieldDecl {
                visibility,
                ty,
                name,
                init,
                span: ty_start.merge(end_span),
            }))
        }
    }

    /// Parse a property accessor (get or set).
    fn parse_property_accessor(&mut self) -> Result<PropertyAccessor, ParseError> {
        let start_span = self.peek().span;

        // Parse accessor kind
        let kind = if self.check_contextual("get") {
            self.advance();
            PropAccessorKind::Get
        } else if self.check_contextual("set") {
            self.advance();
            PropAccessorKind::Set
        } else {
            let span = self.peek().span;
            return Err(ParseError::new(

                ParseErrorKind::InvalidSyntax,

                span,

                "expected 'get' or 'set'",

            ));
        };

        let is_const = self.eat(TokenKind::Const).is_some();
        let attrs = self.parse_func_attrs()?;

        let body = if self.check(TokenKind::LeftBrace) {
            Some(self.parse_block()?)
        } else {
            self.expect(TokenKind::Semicolon)?;
            None
        };

        let span = start_span.merge(
            self.buffer.get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(start_span)
        );

        Ok(PropertyAccessor {
            kind,
            is_const,
            attrs,
            body,
            span,
        })
    }

    /// Parse an interface declaration.
    pub fn parse_interface(&mut self, modifiers: DeclModifiers) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::Interface)?.span;

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        // Optional base interfaces
        let bases = if self.eat(TokenKind::Colon).is_some() {
            let mut bases = Vec::new();
            bases.push(self.parse_ident()?);

            while self.eat(TokenKind::Comma).is_some() {
                bases.push(self.parse_ident()?);
            }
            bases
        } else {
            Vec::new()
        };

        // Check for forward declaration
        if self.eat(TokenKind::Semicolon).is_some() {
            let span = start_span.merge(
                self.buffer.get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span)
            );
            return Ok(Item::Interface(InterfaceDecl {
                modifiers,
                name,
                bases,
                members: Vec::new(),
                span,
            }));
        }

        self.expect(TokenKind::LeftBrace)?;

        // Parse members
        let mut members = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_eof() {
            match self.parse_interface_member() {
                Ok(member) => members.push(member),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                    if self.check(TokenKind::RightBrace) || self.is_eof() {
                        break;
                    }
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(Item::Interface(InterfaceDecl {
            modifiers,
            name,
            bases,
            members,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse an interface member.
    fn parse_interface_member(&mut self) -> Result<InterfaceMember, ParseError> {
        let start_span = self.peek().span;

        let return_type = self.parse_return_type()?;
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        if self.check(TokenKind::LeftBrace) {
            // Virtual property
            self.advance();
            let mut accessors = Vec::new();

            while !self.check(TokenKind::RightBrace) && !self.is_eof() {
                accessors.push(self.parse_property_accessor()?);
            }

            let end_span = self.expect(TokenKind::RightBrace)?.span;

            Ok(InterfaceMember::VirtualProperty(VirtualPropertyDecl {
                visibility: Visibility::Public,
                ty: return_type,
                name,
                accessors,
                span: start_span.merge(end_span),
            }))
        } else {
            // Method signature
            let params = self.parse_function_params()?;
            let is_const = self.eat(TokenKind::Const).is_some();
            let end_span = self.expect(TokenKind::Semicolon)?.span;

            Ok(InterfaceMember::Method(InterfaceMethod {
                return_type,
                name,
                params,
                is_const,
                span: start_span.merge(end_span),
            }))
        }
    }

    /// Parse an enum declaration.
    pub fn parse_enum(&mut self, modifiers: DeclModifiers) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::Enum)?.span;

        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        // Check for forward declaration
        if self.eat(TokenKind::Semicolon).is_some() {
            let span = start_span.merge(
                self.buffer.get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span)
            );
            return Ok(Item::Enum(EnumDecl {
                modifiers,
                name,
                enumerators: Vec::new(),
                span,
            }));
        }

        self.expect(TokenKind::LeftBrace)?;

        let mut enumerators = Vec::new();

        // Parse enumerators
        if !self.check(TokenKind::RightBrace) {
            enumerators.push(self.parse_enumerator()?);

            while self.eat(TokenKind::Comma).is_some() {
                if self.check(TokenKind::RightBrace) {
                    break; // Trailing comma
                }
                enumerators.push(self.parse_enumerator()?);
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(Item::Enum(EnumDecl {
            modifiers,
            name,
            enumerators,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse an enumerator.
    fn parse_enumerator(&mut self) -> Result<Enumerator, ParseError> {
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        let value = if self.eat(TokenKind::Equal).is_some() {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        let span = if let Some(ref v) = value {
            name.span.merge(v.span())
        } else {
            name.span
        };

        Ok(Enumerator { name, value, span })
    }

    /// Parse a namespace declaration.
    pub fn parse_namespace(&mut self) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::Namespace)?.span;

        // Parse namespace path (can be nested: A::B::C)
        let mut path = Vec::new();
        path.push(self.parse_ident()?);

        while self.eat(TokenKind::ColonColon).is_some() {
            path.push(self.parse_ident()?);
        }

        self.expect(TokenKind::LeftBrace)?;

        // Parse namespace contents
        let mut items = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_eof() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                    if self.check(TokenKind::RightBrace) || self.is_eof() {
                        break;
                    }
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(Item::Namespace(NamespaceDecl {
            path,
            items,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a typedef declaration.
    pub fn parse_typedef(&mut self) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::Typedef)?.span;

        let base_type = self.parse_type()?;
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);
        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Item::Typedef(TypedefDecl {
            base_type,
            name,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a funcdef declaration.
    pub fn parse_funcdef(&mut self, modifiers: DeclModifiers) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::FuncDef)?.span;

        let return_type = self.parse_return_type()?;
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        // Optional template parameters (for application-registered template funcdefs)
        // Example: funcdef void Callback<T>(T value);
        // Note: Scripts cannot define template funcdefs, but parser accepts them
        let template_params = if self.check(TokenKind::Less) {
            self.parse_template_param_names()?
        } else {
            Vec::new()
        };

        let params = self.parse_function_params()?;
        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Item::Funcdef(FuncdefDecl {
            modifiers,
            return_type,
            name,
            template_params,
            params,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a mixin declaration.
    pub fn parse_mixin(&mut self) -> Result<Item, ParseError> {
        let start_span = self.eat(TokenKind::Mixin)
            .ok_or_else(|| {
                let span = self.peek().span;
                ParseError::new(
                    ParseErrorKind::ExpectedDeclaration,
                    span,
                    "expected 'mixin'",
                )
            })?
            .span;

        let class_item = self.parse_class(DeclModifiers::new(), Visibility::Public)?;

        if let Item::Class(class) = class_item {
            let span = start_span.merge(class.span);
            Ok(Item::Mixin(MixinDecl { class, span }))
        } else {
            unreachable!()
        }
    }

    /// Parse an import declaration.
    pub fn parse_import(&mut self) -> Result<Item, ParseError> {
        let start_span = self.expect(TokenKind::Import)?.span;

        let return_type = self.parse_return_type()?;
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);
        let params = self.parse_function_params()?;
        let attrs = self.parse_func_attrs()?;

        // Expect 'from' keyword
        if !self.check_contextual("from") {
            let span = self.peek().span;
            return Err(ParseError::new(

                ParseErrorKind::ExpectedToken,

                span,

                "expected 'from' keyword in import declaration",

            ));
        }
        self.advance();

        // Parse module string
        let module_token = self.expect(TokenKind::StringLiteral)?;
        let module = module_token.lexeme.trim_matches('"').to_string();

        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Item::Import(ImportDecl {
            return_type,
            name,
            params,
            attrs,
            module,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse an identifier.
    fn parse_ident(&mut self) -> Result<Ident, ParseError> {
        let token = self.expect(TokenKind::Identifier)?;
        Ok(Ident::new(token.lexeme, token.span))
    }

    /// Parse template parameter names for class/funcdef declarations.
    ///
    /// This is used for application-registered template types.
    /// Scripts cannot define template classes/functions, but the parser accepts them.
    ///
    /// Example: `<T>`, `<T, U>`, `<K, V>`
    fn parse_template_param_names(&mut self) -> Result<Vec<Ident>, ParseError> {
        self.expect(TokenKind::Less)?;

        let mut params = Vec::new();

        if !self.check(TokenKind::Greater) {
            params.push(self.parse_ident()?);

            while self.eat(TokenKind::Comma).is_some() {
                params.push(self.parse_ident()?);
            }
        }

        self.expect(TokenKind::Greater)?;

        Ok(params)
    }
}