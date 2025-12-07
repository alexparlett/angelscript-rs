//! Type parsing functions for AngelScript.
//!
//! Implements parsing of type expressions including:
//! - Primitive types
//! - User-defined types
//! - Scoped types (Namespace::Type)
//! - Template types (array<T>)
//! - Type modifiers (const, references, handles)
//! - Array and handle suffixes

use crate::ast::{Ident, ParseError, ParseErrorKind, RefKind, Scope};
use crate::ast::types::*;
use crate::lexer::{self, Span, TokenKind};
use super::parser::Parser;

impl<'ast> Parser<'ast> {
    /// Parse a complete type expression.
    ///
    /// Grammar: `'const'? SCOPE DATATYPE TEMPLTYPELIST? ( ('[' ']') | ('@' 'const'?) )*`
    ///
    /// Examples:
    /// - `int`
    /// - `const array<int>[]`
    /// - `Namespace::MyClass@`
    /// - `const MyClass@ const`
    pub fn parse_type(&mut self) -> Result<TypeExpr<'ast>, ParseError> {
        let start_span = self.peek().span;

        // Leading const (makes the object const, not the handle)
        let is_const = self.eat(TokenKind::Const).is_some();

        // Optional scope
        let scope = self.parse_optional_scope()?;

        // Base type
        let base = self.parse_type_base()?;

        // Template arguments
        let template_args = if self.check(TokenKind::Less) {
            self.parse_template_args()?
        } else {
            &[]
        };

        // Type suffixes (arrays and handles)
        let suffixes = self.parse_type_suffixes()?;

        let end_span = if !suffixes.is_empty() {
            self.buffer.get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(start_span)
        } else if !template_args.is_empty() {
            self.buffer.get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(start_span)
        } else {
            self.buffer.get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(start_span)
        };

        let span = start_span.merge(end_span);

        Ok(TypeExpr::new(is_const, scope, base, template_args, suffixes, span))
    }

    /// Parse a return type (type with optional & reference).
    ///
    /// Grammar: `TYPE '&'?`
    ///
    /// Examples:
    /// - `void`
    /// - `int&`
    /// - `const string&`
    pub fn parse_return_type(&mut self) -> Result<ReturnType<'ast>, ParseError> {
        let ty = self.parse_type()?;
        let is_ref = self.eat(TokenKind::Amp).is_some();
        let span = if is_ref {
            ty.span.merge(self.buffer.get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(ty.span))
        } else {
            ty.span
        };

        Ok(ReturnType::new(ty, is_ref, span))
    }

    /// Parse a parameter type (type with optional & and flow modifiers).
    ///
    /// Grammar: `TYPE ('&' ('in' | 'out' | 'inout')?)?`
    ///
    /// Examples:
    /// - `int`
    /// - `int&`
    /// - `int& in`
    /// - `int& out`
    /// - `int& inout`
    pub fn parse_param_type(&mut self) -> Result<ParamType<'ast>, ParseError> {
        let ty = self.parse_type()?;

        let ref_kind = if self.eat(TokenKind::Amp).is_some() {
            // Check for flow direction
            if self.eat(TokenKind::In).is_some() {
                RefKind::RefIn
            } else if self.eat(TokenKind::Out).is_some() {
                RefKind::RefOut
            } else if self.eat(TokenKind::InOut).is_some() {
                RefKind::RefInOut
            } else {
                RefKind::Ref
            }
        } else {
            RefKind::None
        };

        let span = ty.span.merge(
            self.buffer.get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(ty.span)
        );

        Ok(ParamType::new(ty, ref_kind, span))
    }

    /// Parse an optional scope prefix.
    ///
    /// Grammar: `'::'? (IDENTIFIER '::')* (IDENTIFIER TEMPLTYPELIST? '::')?`
    ///
    /// Examples:
    /// - `::` (absolute, empty)
    /// - `Namespace::`
    /// - `::Namespace::SubSpace::`
    /// - `Container<T>::`
    pub fn parse_optional_scope(&mut self) -> Result<Option<Scope<'ast>>, ParseError> {
        let start_span = self.peek().span;

        // Check for leading ::
        let is_absolute = self.eat(TokenKind::ColonColon).is_some();

        let mut segments = bumpalo::collections::Vec::new_in(self.arena);
        let mut last_span = start_span;

        // Parse namespace segments
        loop {
            // Look ahead to see if this is a scope segment
            if self.check(TokenKind::Identifier) {
                // Need to check if followed by ::
                let lookahead_1 = self.peek_nth(1).kind;
                let lookahead_2 = if lookahead_1 == TokenKind::Less {
                    // Could be template: Type<...>::
                    // For now, don't parse templated scopes (simplified)
                    // We'll just check if there's :: after the identifier
                    TokenKind::Less
                } else {
                    lookahead_1
                };

                if lookahead_2 == TokenKind::ColonColon {
                    // This is a scope segment
                    let ident_token = self.advance();
                    let ident = Ident::new(ident_token.lexeme, ident_token.span);
                    last_span = ident_token.span;

                    if self.check(TokenKind::Less) {
                        let span = self.peek().span;
                        return Err(crate::ast::ParseError::new(
                            crate::ast::ParseErrorKind::NotImplemented,
                            span,
                            "template arguments in scope resolution are not supported"
                        ));
                    }

                    self.expect(TokenKind::ColonColon)?;
                    last_span = self.buffer.get(self.position.saturating_sub(1))
                        .map(|t| t.span)
                        .unwrap_or(last_span);

                    segments.push(ident);
                } else {
                    // Not a scope, stop here
                    break;
                }
            } else {
                break;
            }
        }

        // If we have an absolute marker or segments, create a scope
        if is_absolute || !segments.is_empty() {
            let span = start_span.merge(last_span);
            let segments_slice = segments.into_bump_slice();
            let scope = Scope::new(is_absolute, segments_slice, span);
            // SAFETY: Scope borrows from the arena which outlives the parser.
            // We transmute the lifetime from the local borrow to 'ast which is sound
            // because the arena (&'ast Bump) lives for 'ast.
            // The scope is already allocated in the arena, so no transmute needed
            Ok(Some(scope))
        } else {
            Ok(None)
        }
    }

    /// Parse the base type (primitive, identifier, auto, or ?).
    ///
    /// Grammar: `'class'? IDENTIFIER | PRIMTYPE | '?' | 'auto'`
    ///
    /// The optional `class` keyword is used in FFI template parameter declarations
    /// like `array<class T>` where `class T` declares a type parameter named `T`.
    fn parse_type_base(&mut self) -> Result<TypeBase<'ast>, ParseError> {
        let token = *self.peek();

        match token.kind {
            // Primitive types
            TokenKind::Void => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Void))
            }
            TokenKind::Bool => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Bool))
            }
            TokenKind::Int => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Int))
            }
            TokenKind::Int8 => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Int8))
            }
            TokenKind::Int16 => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Int16))
            }
            TokenKind::Int64 => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Int64))
            }
            TokenKind::UInt => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::UInt))
            }
            TokenKind::UInt8 => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::UInt8))
            }
            TokenKind::UInt16 => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::UInt16))
            }
            TokenKind::UInt64 => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::UInt64))
            }
            TokenKind::Float => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Float))
            }
            TokenKind::Double => {
                self.advance();
                Ok(TypeBase::Primitive(PrimitiveType::Double))
            }

            // Auto type
            TokenKind::Auto => {
                self.advance();
                Ok(TypeBase::Auto)
            }

            // Unknown type (?)
            TokenKind::Question => {
                self.advance();
                Ok(TypeBase::Unknown)
            }

            // Optional 'class' keyword before identifier (for FFI template params)
            // e.g., `array<class T>` where `class T` is a type parameter declaration
            TokenKind::Class => {
                self.advance();
                let ident_token = self.expect(TokenKind::Identifier)?;
                Ok(TypeBase::TemplateParam(Ident::new(ident_token.lexeme, ident_token.span)))
            }

            // User-defined type (identifier)
            TokenKind::Identifier => {
                let ident_token = self.advance();
                Ok(TypeBase::Named(Ident::new(ident_token.lexeme, ident_token.span)))
            }

            _ => {
                Err(ParseError::new(
                    ParseErrorKind::ExpectedType,
                    token.span,
                    format!("expected type, found {}", token.kind),
                ))
            }
        }
    }

    /// Parse template argument list.
    ///
    /// Grammar: `'<' TYPE (',' TYPE)* '>'`
    ///
    /// Note: Handles >> splitting for nested templates.
    fn parse_template_args(&mut self) -> Result<&'ast [TypeExpr<'ast>], ParseError> {
        self.expect(TokenKind::Less)?;

        let mut args = bumpalo::collections::Vec::new_in(self.arena);

        // Parse first argument
        if !self.is_template_close() {
            args.push(self.parse_type()?);

            // Parse remaining arguments
            while self.eat(TokenKind::Comma).is_some() {
                args.push(self.parse_type()?);
            }
        }

        // Expect closing > (handles >>, >>>, etc. automatically via splitting)
        self.expect_template_close()?;

        Ok(args.into_bump_slice())
    }

    /// Check if we're at a template closing token (>, >>, or >>>).
    fn is_template_close(&mut self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Greater | TokenKind::GreaterGreater | TokenKind::GreaterGreaterGreater
        )
    }

    /// Expect and consume a template closing >.
    ///
    /// This handles nested templates by automatically splitting >>, >>>, etc.
    /// into individual > tokens. This allows arbitrary nesting depth without
    /// special handling for each level.
    fn expect_template_close(&mut self) -> Result<(), ParseError> {
        if self.check(TokenKind::Greater) {
            // Simple case: single >
            self.advance();
            Ok(())
        } else if self.check(TokenKind::GreaterGreater) {
            // Split >> into > + >, consume one
            self.split_greater_greater();
            self.advance();
            Ok(())
        } else if self.check(TokenKind::GreaterGreaterGreater) {
            // Split >>> into > + > + >, consume one
            self.split_greater_greater_greater();
            self.advance();
            Ok(())
        } else {
            let span = self.peek().span;
            Err(ParseError::new(
                ParseErrorKind::ExpectedToken,
                span,
                "expected '>' to close template arguments",
            ))
        }
    }

    /// Split a >>> token into three > tokens for template parsing.
    ///
    /// This is needed for deeply nested templates like `array<array<array<int>>>`
    /// which produce a >>> token that should be treated as three closing angle brackets.
    fn split_greater_greater_greater(&mut self) {
        if self.check(TokenKind::GreaterGreaterGreater) {
            let token = self.buffer[self.position];
            
            // Replace >>> with three > tokens
            let first_greater = lexer::Token {
                kind: TokenKind::Greater,
                lexeme: ">",
                span: Span::new(token.span.line, token.span.col, 1),
            };
            let second_greater = lexer::Token {
                kind: TokenKind::Greater,
                lexeme: ">",
                span: Span::new(token.span.line, token.span.col + 1, 1),
            };
            let third_greater = lexer::Token {
                kind: TokenKind::Greater,
                lexeme: ">",
                span: Span::new(token.span.line, token.span.col + 2, 1),
            };

            // Replace the >>> token with three > tokens
            self.buffer[self.position] = first_greater;
            self.buffer.insert(self.position + 1, second_greater);
            self.buffer.insert(self.position + 2, third_greater);
        }
    }

    /// Split a >> token into two > tokens for template parsing.
    ///
    /// This is needed because nested templates like `array<array<int>>`
    /// produce a >> token that should be treated as two closing angle brackets.
    fn split_greater_greater(&mut self) {
        if self.check(TokenKind::GreaterGreater) {
            let token = self.buffer[self.position];

            // Replace >> with two > tokens
            let first_greater = lexer::Token {
                kind: TokenKind::Greater,
                lexeme: ">",
                span: Span::new(token.span.line, token.span.col, 1),
            };
            let second_greater = lexer::Token {
                kind: TokenKind::Greater,
                lexeme: ">",
                span: Span::new(token.span.line, token.span.col + 1, 1),
            };

            // Replace the >> token with two > tokens
            self.buffer[self.position] = first_greater;
            self.buffer.insert(self.position + 1, second_greater);
        }
    }

    /// Parse type suffixes (arrays and handles).
    ///
    /// Grammar: `( ('[' ']') | ('@' 'const'?) )*`
    ///
    /// Examples:
    /// - `[]` - array
    /// - `@` - handle
    /// - `@ const` - const handle
    /// - `[]@` - array of handles
    /// - `[]@ const` - const handle to array
    fn parse_type_suffixes(&mut self) -> Result<&'ast [TypeSuffix], ParseError> {
        let mut suffixes = bumpalo::collections::Vec::new_in(self.arena);

        loop {
            if self.check(TokenKind::At) {
                // Handle suffix: @ or @ const
                self.advance();
                let is_const = self.eat(TokenKind::Const).is_some();
                suffixes.push(TypeSuffix::Handle { is_const });
            } else {
                // No more suffixes
                break;
            }
        }

        Ok(suffixes.into_bump_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_primitive_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(!ty.is_const);
        assert!(matches!(ty.base, TypeBase::Primitive(PrimitiveType::Int)));
        assert!(ty.suffixes.is_empty());
    }

    #[test]
    fn parse_const_primitive() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const int", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert!(matches!(ty.base, TypeBase::Primitive(PrimitiveType::Int)));
    }

    #[test]
    fn parse_named_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Named(_)));
    }

    #[test]
    fn parse_handle() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass@", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 1);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Handle { is_const: false }));
    }

    #[test]
    fn parse_const_handle() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass@ const", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 1);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Handle { is_const: true }));
    }

    #[test]
    fn parse_template_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<int>", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
    }

    #[test]
    fn parse_complex_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const array<int>@ const", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.suffixes.len(), 1);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Handle { is_const: true }));
    }

    #[test]
    fn parse_return_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int&", &arena);
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(ret_ty.is_ref);
        assert!(matches!(ret_ty.ty.base, TypeBase::Primitive(PrimitiveType::Int)));
    }

    #[test]
    fn parse_param_type_in() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int& in", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefIn));
    }

    #[test]
    fn parse_param_type_out() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int& out", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefOut));
    }

    #[test]
    fn parse_param_type_inout() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int& inout", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefInOut));
    }

    #[test]
    fn parse_scoped_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Namespace::MyClass", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(!scope.is_absolute);
        assert_eq!(scope.segments.len(), 1);
    }

    #[test]
    fn parse_absolute_scoped_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::GlobalType", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
    }

    #[test]
    fn parse_nested_template_with_double_greater() {
        // This tests the >> splitting for nested templates
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<array<int>>", &arena);
        let ty = parser.parse_type().unwrap();
        
        // Should parse as array with template arg of (array with template arg of int)
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
        
        let inner_ty = &ty.template_args[0];
        assert!(matches!(inner_ty.base, TypeBase::Named(_)));
        assert_eq!(inner_ty.template_args.len(), 1);
        
        let innermost_ty = &inner_ty.template_args[0];
        assert!(matches!(innermost_ty.base, TypeBase::Primitive(PrimitiveType::Int)));
    }

    #[test]
    fn parse_triple_nested_template() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<array<array<int>>>", &arena);
        let ty = parser.parse_type().unwrap();
        
        // Should handle even deeper nesting
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
    }

    #[test]
    fn parse_deeply_nested_template_four_levels() {
        // Test the exact case from the user: array<array<weakref<Foo<string>>>>
        // All these are registered application types, parsed as Named
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<array<weakref<Foo<string>>>>", &arena);
        let ty = parser.parse_type().unwrap();
        
        // Verify outer array
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
        
        // Verify second level array
        let level2 = &ty.template_args[0];
        assert!(matches!(level2.base, TypeBase::Named(_)));
        assert_eq!(level2.template_args.len(), 1);
        
        // Verify third level weakref
        let level3 = &level2.template_args[0];
        assert!(matches!(level3.base, TypeBase::Named(_)));
        assert_eq!(level3.template_args.len(), 1);
        
        // Verify fourth level Foo
        let level4 = &level3.template_args[0];
        assert!(matches!(level4.base, TypeBase::Named(_)));
        assert_eq!(level4.template_args.len(), 1);
        
        // Verify innermost string (also a registered type)
        let level5 = &level4.template_args[0];
        assert!(matches!(level5.base, TypeBase::Named(_)));
        
        // Verify no extra nesting
        assert!(level5.template_args.is_empty());
    }

    #[test]
    fn parse_five_level_nested_template() {
        // Test even deeper: array<dict<array<map<vector<int>>>>>
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<dict<array<map<vector<int>>>>>", &arena);
        let ty = parser.parse_type().unwrap();
        
        // Just verify it parses without error and has correct structure
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
        
        // Verify the nesting goes at least 3 levels deep
        let level2 = &ty.template_args[0];
        assert_eq!(level2.template_args.len(), 1);
        
        let level3 = &level2.template_args[0];
        assert_eq!(level3.template_args.len(), 1);
        
        // Continue to deeper levels
        let level4 = &level3.template_args[0];
        assert_eq!(level4.template_args.len(), 1);

        let level5 = &level4.template_args[0];
        assert_eq!(level5.template_args.len(), 1);

        // Verify 6th level (int inside vector) is a primitive
        let level6 = &level5.template_args[0];
        assert!(matches!(level6.base, TypeBase::Primitive(PrimitiveType::Int)));
    }

    #[test]
    fn parse_extremely_deep_nesting() {
        // Test 10 levels deep to verify arbitrary nesting really works!
        // a<b<c<d<e<f<g<h<i<j>>>>>>>>>>
        //
        // This demonstrates that our recursive splitting approach handles
        // ANY depth correctly. Each level's expect_template_close() splits
        // compound tokens (>>, >>>) as needed and consumes one >,
        // leaving the rest for parent levels.
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a<b<c<d<e<f<g<h<i<j>>>>>>>>>>", &arena);
        let ty = parser.parse_type().unwrap();

        // Verify outer type
        assert!(matches!(ty.base, TypeBase::Named(_)));

        // Walk down 9 levels
        let mut current = &ty;
        for level in 1..=9 {
            assert_eq!(current.template_args.len(), 1, "Level {} should have 1 arg", level);
            current = &current.template_args[0];
        }

        // Level 10 should have no args (it's 'j' with no template)
        assert!(current.template_args.is_empty(), "Innermost type should have no args");
    }

    // ========================================================================
    // Additional Primitive Type Tests
    // ========================================================================

    #[test]
    fn parse_all_primitive_types() {
        let types = vec![
            ("void", PrimitiveType::Void),
            ("bool", PrimitiveType::Bool),
            ("int", PrimitiveType::Int),
            ("int8", PrimitiveType::Int8),
            ("int16", PrimitiveType::Int16),
            ("int64", PrimitiveType::Int64),
            ("uint", PrimitiveType::UInt),
            ("uint8", PrimitiveType::UInt8),
            ("uint16", PrimitiveType::UInt16),
            ("uint64", PrimitiveType::UInt64),
            ("float", PrimitiveType::Float),
            ("double", PrimitiveType::Double),
        ];

        for (type_str, expected) in types {
            let arena = bumpalo::Bump::new();
            let mut parser = Parser::new(type_str, &arena);
            let ty = parser.parse_type().unwrap();
            match ty.base {
                TypeBase::Primitive(prim) => {
                    assert!(
                        std::mem::discriminant(&prim) == std::mem::discriminant(&expected),
                        "Failed for type: {}",
                        type_str
                    );
                }
                _ => panic!("Expected primitive type for: {}", type_str),
            }
        }
    }

    #[test]
    fn parse_auto_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("auto", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Auto));
    }

    #[test]
    fn parse_unknown_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("?", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Unknown));
    }

    // ========================================================================
    // Scope Tests
    // ========================================================================

    #[test]
    fn parse_scope_empty_global() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::int", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
        assert_eq!(scope.segments.len(), 0);
    }

    #[test]
    fn parse_scope_multi_segment() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("A::B::C::Type", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(!scope.is_absolute);
        assert_eq!(scope.segments.len(), 3);
    }

    #[test]
    fn parse_scope_absolute_multi_segment() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::A::B::Type", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
        assert_eq!(scope.segments.len(), 2);
    }

    // ========================================================================
    // Template Tests
    // ========================================================================

    #[test]
    fn parse_template_empty() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Container<>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 0);
    }

    #[test]
    fn parse_template_multiple_args() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("map<string, int>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 2);
    }

    #[test]
    fn parse_template_with_const() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<const int>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 1);
        assert!(ty.template_args[0].is_const);
    }

    #[test]
    fn parse_template_with_handle() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<Foo@>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.template_args[0].suffixes.len(), 1);
    }

    #[test]
    fn parse_template_complex_args() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("map<const string@, array<int>>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 2);
    }

    // ========================================================================
    // Return Type Tests
    // ========================================================================

    #[test]
    fn parse_return_type_no_ref() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int", &arena);
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(!ret_ty.is_ref);
    }

    #[test]
    fn parse_return_type_with_ref() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("string&", &arena);
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(ret_ty.is_ref);
    }

    #[test]
    fn parse_return_type_handle_ref() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Foo@&", &arena);
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(ret_ty.is_ref);
        assert_eq!(ret_ty.ty.suffixes.len(), 1);
    }

    // ========================================================================
    // Param Type Tests
    // ========================================================================

    #[test]
    fn parse_param_type_no_ref() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::None));
    }

    #[test]
    fn parse_param_type_ref() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int&", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::Ref));
    }

    #[test]
    fn parse_param_type_ref_in() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const string& in", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefIn));
        assert!(param_ty.ty.is_const);
    }

    #[test]
    fn parse_param_type_ref_out() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int& out", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefOut));
    }

    #[test]
    fn parse_param_type_ref_inout() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<int>& inout", &arena);
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefInOut));
    }

    // ========================================================================
    // Complex Combinations
    // ========================================================================

    #[test]
    fn parse_const_scoped_template_handle() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const A::B::Container<int>@", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert!(ty.scope.is_some());
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.suffixes.len(), 1);
    }

    #[test]
    fn parse_global_scoped_template() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::std::vector<string>@", &arena);
        let ty = parser.parse_type().unwrap();
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.suffixes.len(), 1);
    }

    #[test]
    fn parse_all_features_combined() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const ::A::B::map<string, int>@ const", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert!(ty.scope.is_some());
        assert_eq!(ty.template_args.len(), 2);
        assert_eq!(ty.suffixes.len(), 1);
    }

    // ========================================================================
    // Template Edge Cases
    // ========================================================================

    #[test]
    fn parse_template_with_shift_operator_lookalike() {
        // Make sure we properly handle >> that's NOT a template close
        // This is parsed as array<array<int>> not array<(array<int >> something)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<array<int>>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.template_args[0].template_args.len(), 1);
    }

    #[test]
    fn parse_template_mixed_nesting_levels() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("map<int, array<string>>", &arena);
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 2);
        // Second arg is nested template
        assert_eq!(ty.template_args[1].template_args.len(), 1);
    }

    // ========================================================================
    // Const Variations
    // ========================================================================

    #[test]
    fn parse_const_before_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const int", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
    }

    #[test]
    fn parse_const_handle_variations() {
        // const Type@ - const object, non-const handle
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const Foo@", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(!is_const),
            _ => panic!("Expected non-const handle"),
        }

        // Type@ const - non-const object, const handle
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Foo@ const", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(!ty.is_const);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(*is_const),
            _ => panic!("Expected const handle"),
        }

        // const Type@ const - const object, const handle
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("const Foo@ const", &arena);
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(*is_const),
            _ => panic!("Expected const handle"),
        }
    }
}
