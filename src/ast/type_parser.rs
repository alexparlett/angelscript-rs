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

impl<'src> Parser<'src> {
    /// Parse a complete type expression.
    ///
    /// Grammar: `'const'? SCOPE DATATYPE TEMPLTYPELIST? ( ('[' ']') | ('@' 'const'?) )*`
    ///
    /// Examples:
    /// - `int`
    /// - `const array<int>[]`
    /// - `Namespace::MyClass@`
    /// - `const MyClass@ const`
    pub fn parse_type(&mut self) -> Result<TypeExpr, ParseError> {
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
            Vec::new()
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
    pub fn parse_return_type(&mut self) -> Result<ReturnType, ParseError> {
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
    pub fn parse_param_type(&mut self) -> Result<ParamType, ParseError> {
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
    pub fn parse_optional_scope(&mut self) -> Result<Option<Scope>, ParseError> {
        let start_span = self.peek().span;

        // Check for leading ::
        let is_absolute = self.eat(TokenKind::ColonColon).is_some();

        let mut segments = Vec::new();
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

                    // Skip template args if present (simplified for now)
                    // In a full implementation, we'd parse template args here
                    if self.check(TokenKind::Less) {
                        // TODO: Properly parse template arguments in scope resolution
                        // For now, report error - not yet implemented
                        let span = self.peek().span;
                        return Err(crate::ast::ParseError::new(
                            crate::ast::ParseErrorKind::NotImplemented,
                            span,
                            "template arguments in scope resolution are not yet implemented"
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
            Ok(Some(Scope::new(is_absolute, segments, span)))
        } else {
            Ok(None)
        }
    }

    /// Parse the base type (primitive, identifier, auto, or ?).
    ///
    /// Grammar: `IDENTIFIER | PRIMTYPE | '?' | 'auto'`
    fn parse_type_base(&mut self) -> Result<TypeBase, ParseError> {
        let token = self.peek().clone();

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
    fn parse_template_args(&mut self) -> Result<Vec<TypeExpr>, ParseError> {
        self.expect(TokenKind::Less)?;

        let mut args = Vec::new();

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

        Ok(args)
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
            let token = self.buffer[self.position].clone();
            
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
            let token = self.buffer[self.position].clone();

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
    fn parse_type_suffixes(&mut self) -> Result<Vec<TypeSuffix>, ParseError> {
        let mut suffixes = Vec::new();

        loop {
            if self.check(TokenKind::LeftBracket) {
                // Array suffix: []
                self.advance();
                self.expect(TokenKind::RightBracket)?;
                suffixes.push(TypeSuffix::Array);
            } else if self.check(TokenKind::At) {
                // Handle suffix: @ or @ const
                self.advance();
                let is_const = self.eat(TokenKind::Const).is_some();
                suffixes.push(TypeSuffix::Handle { is_const });
            } else {
                // No more suffixes
                break;
            }
        }

        Ok(suffixes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_primitive_type() {
        let mut parser = Parser::new("int");
        let ty = parser.parse_type().unwrap();
        assert!(!ty.is_const);
        assert!(matches!(ty.base, TypeBase::Primitive(PrimitiveType::Int)));
        assert!(ty.suffixes.is_empty());
    }

    #[test]
    fn parse_const_primitive() {
        let mut parser = Parser::new("const int");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert!(matches!(ty.base, TypeBase::Primitive(PrimitiveType::Int)));
    }

    #[test]
    fn parse_named_type() {
        let mut parser = Parser::new("MyClass");
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Named(_)));
    }

    #[test]
    fn parse_handle() {
        let mut parser = Parser::new("MyClass@");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 1);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Handle { is_const: false }));
    }

    #[test]
    fn parse_const_handle() {
        let mut parser = Parser::new("MyClass@ const");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 1);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Handle { is_const: true }));
    }

    #[test]
    fn parse_array() {
        let mut parser = Parser::new("int[]");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 1);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Array));
    }

    #[test]
    fn parse_array_handle() {
        let mut parser = Parser::new("int[]@");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 2);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Array));
        assert!(matches!(ty.suffixes[1], TypeSuffix::Handle { is_const: false }));
    }

    #[test]
    fn parse_template_type() {
        let mut parser = Parser::new("array<int>");
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
    }

    #[test]
    fn parse_complex_type() {
        let mut parser = Parser::new("const array<int>[]@ const");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.suffixes.len(), 2);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Array));
        assert!(matches!(ty.suffixes[1], TypeSuffix::Handle { is_const: true }));
    }

    #[test]
    fn parse_return_type() {
        let mut parser = Parser::new("int&");
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(ret_ty.is_ref);
        assert!(matches!(ret_ty.ty.base, TypeBase::Primitive(PrimitiveType::Int)));
    }

    #[test]
    fn parse_param_type_in() {
        let mut parser = Parser::new("int& in");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefIn));
    }

    #[test]
    fn parse_param_type_out() {
        let mut parser = Parser::new("int& out");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefOut));
    }

    #[test]
    fn parse_param_type_inout() {
        let mut parser = Parser::new("int& inout");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefInOut));
    }

    #[test]
    fn parse_scoped_type() {
        let mut parser = Parser::new("Namespace::MyClass");
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(!scope.is_absolute);
        assert_eq!(scope.segments.len(), 1);
    }

    #[test]
    fn parse_absolute_scoped_type() {
        let mut parser = Parser::new("::GlobalType");
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
    }

    #[test]
    fn parse_nested_template_with_double_greater() {
        // This tests the >> splitting for nested templates
        let mut parser = Parser::new("array<array<int>>");
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
        let mut parser = Parser::new("array<array<array<int>>>");
        let ty = parser.parse_type().unwrap();
        
        // Should handle even deeper nesting
        assert!(matches!(ty.base, TypeBase::Named(_)));
        assert_eq!(ty.template_args.len(), 1);
    }

    #[test]
    fn parse_deeply_nested_template_four_levels() {
        // Test the exact case from the user: array<array<weakref<Foo<string>>>>
        // All these are registered application types, parsed as Named
        let mut parser = Parser::new("array<array<weakref<Foo<string>>>>");
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
        let mut parser = Parser::new("array<dict<array<map<vector<int>>>>>");
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
        let mut parser = Parser::new("a<b<c<d<e<f<g<h<i<j>>>>>>>>>>");
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
            let mut parser = Parser::new(type_str);
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
        let mut parser = Parser::new("auto");
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Auto));
    }

    #[test]
    fn parse_unknown_type() {
        let mut parser = Parser::new("?");
        let ty = parser.parse_type().unwrap();
        assert!(matches!(ty.base, TypeBase::Unknown));
    }

    // ========================================================================
    // Scope Tests
    // ========================================================================

    #[test]
    fn parse_scope_empty_global() {
        let mut parser = Parser::new("::int");
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
        assert_eq!(scope.segments.len(), 0);
    }

    #[test]
    fn parse_scope_multi_segment() {
        let mut parser = Parser::new("A::B::C::Type");
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(!scope.is_absolute);
        assert_eq!(scope.segments.len(), 3);
    }

    #[test]
    fn parse_scope_absolute_multi_segment() {
        let mut parser = Parser::new("::A::B::Type");
        let ty = parser.parse_type().unwrap();
        assert!(ty.scope.is_some());
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
        assert_eq!(scope.segments.len(), 2);
    }

    // ========================================================================
    // Suffix Combination Tests
    // ========================================================================

    #[test]
    fn parse_multiple_array_suffixes() {
        let mut parser = Parser::new("int[][]");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 2);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Array));
        assert!(matches!(ty.suffixes[1], TypeSuffix::Array));
    }

    #[test]
    fn parse_handle_then_array() {
        let mut parser = Parser::new("int@[]");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 2);
        assert!(matches!(ty.suffixes[0], TypeSuffix::Handle { .. }));
        assert!(matches!(ty.suffixes[1], TypeSuffix::Array));
    }

    #[test]
    fn parse_const_handle_array() {
        let mut parser = Parser::new("int@ const[]");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 2);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(*is_const),
            _ => panic!("Expected const handle"),
        }
        assert!(matches!(ty.suffixes[1], TypeSuffix::Array));
    }

    #[test]
    fn parse_complex_suffix_chain() {
        let mut parser = Parser::new("int[]@[]@ const");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.suffixes.len(), 4);
    }

    // ========================================================================
    // Template Tests
    // ========================================================================

    #[test]
    fn parse_template_empty() {
        let mut parser = Parser::new("Container<>");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 0);
    }

    #[test]
    fn parse_template_multiple_args() {
        let mut parser = Parser::new("map<string, int>");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 2);
    }

    #[test]
    fn parse_template_with_const() {
        let mut parser = Parser::new("array<const int>");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 1);
        assert!(ty.template_args[0].is_const);
    }

    #[test]
    fn parse_template_with_handle() {
        let mut parser = Parser::new("array<Foo@>");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.template_args[0].suffixes.len(), 1);
    }

    #[test]
    fn parse_template_complex_args() {
        let mut parser = Parser::new("map<const string@, array<int>[]>");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 2);
    }

    // ========================================================================
    // Return Type Tests
    // ========================================================================

    #[test]
    fn parse_return_type_no_ref() {
        let mut parser = Parser::new("int");
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(!ret_ty.is_ref);
    }

    #[test]
    fn parse_return_type_with_ref() {
        let mut parser = Parser::new("string&");
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(ret_ty.is_ref);
    }

    #[test]
    fn parse_return_type_handle_ref() {
        let mut parser = Parser::new("Foo@&");
        let ret_ty = parser.parse_return_type().unwrap();
        assert!(ret_ty.is_ref);
        assert_eq!(ret_ty.ty.suffixes.len(), 1);
    }

    // ========================================================================
    // Param Type Tests
    // ========================================================================

    #[test]
    fn parse_param_type_no_ref() {
        let mut parser = Parser::new("int");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::None));
    }

    #[test]
    fn parse_param_type_ref() {
        let mut parser = Parser::new("int&");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::Ref));
    }

    #[test]
    fn parse_param_type_ref_in() {
        let mut parser = Parser::new("const string& in");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefIn));
        assert!(param_ty.ty.is_const);
    }

    #[test]
    fn parse_param_type_ref_out() {
        let mut parser = Parser::new("int& out");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefOut));
    }

    #[test]
    fn parse_param_type_ref_inout() {
        let mut parser = Parser::new("array<int>& inout");
        let param_ty = parser.parse_param_type().unwrap();
        assert!(matches!(param_ty.ref_kind, RefKind::RefInOut));
    }

    // ========================================================================
    // Complex Combinations
    // ========================================================================

    #[test]
    fn parse_const_scoped_template_handle() {
        let mut parser = Parser::new("const A::B::Container<int>@");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert!(ty.scope.is_some());
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.suffixes.len(), 1);
    }

    #[test]
    fn parse_global_scoped_template_array() {
        let mut parser = Parser::new("::std::vector<string>[]");
        let ty = parser.parse_type().unwrap();
        let scope = ty.scope.unwrap();
        assert!(scope.is_absolute);
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.suffixes.len(), 1);
    }

    #[test]
    fn parse_all_features_combined() {
        let mut parser = Parser::new("const ::A::B::map<string, int>[]@ const");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        assert!(ty.scope.is_some());
        assert_eq!(ty.template_args.len(), 2);
        assert_eq!(ty.suffixes.len(), 2);
    }

    // ========================================================================
    // Template Edge Cases
    // ========================================================================

    #[test]
    fn parse_template_with_shift_operator_lookalike() {
        // Make sure we properly handle >> that's NOT a template close
        // This is parsed as array<array<int>> not array<(array<int >> something)
        let mut parser = Parser::new("array<array<int>>");
        let ty = parser.parse_type().unwrap();
        assert_eq!(ty.template_args.len(), 1);
        assert_eq!(ty.template_args[0].template_args.len(), 1);
    }

    #[test]
    fn parse_template_mixed_nesting_levels() {
        let mut parser = Parser::new("map<int, array<string>>");
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
        let mut parser = Parser::new("const int");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
    }

    #[test]
    fn parse_const_handle_variations() {
        // const Type@ - const object, non-const handle
        let mut parser = Parser::new("const Foo@");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(!is_const),
            _ => panic!("Expected non-const handle"),
        }

        // Type@ const - non-const object, const handle
        let mut parser = Parser::new("Foo@ const");
        let ty = parser.parse_type().unwrap();
        assert!(!ty.is_const);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(*is_const),
            _ => panic!("Expected const handle"),
        }

        // const Type@ const - const object, const handle
        let mut parser = Parser::new("const Foo@ const");
        let ty = parser.parse_type().unwrap();
        assert!(ty.is_const);
        match &ty.suffixes[0] {
            TypeSuffix::Handle { is_const } => assert!(*is_const),
            _ => panic!("Expected const handle"),
        }
    }
}
