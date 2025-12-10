//! Type resolution for converting AST type expressions to semantic DataTypes.
//!
//! This module provides [`TypeResolver`], which converts AST [`TypeExpr`] nodes
//! into semantic [`DataType`] values using the compilation context's namespace-aware
//! resolution.
//!
//! ## Features
//!
//! - Primitive type resolution (int, float, bool, void, etc.)
//! - Named type resolution via O(1) scope lookup
//! - Qualified type resolution (Namespace::Type)
//! - Type modifiers (const, handle @, handle-to-const @ const)
//! - Reference modifiers (&in, &out, &inout) for parameters
//!
//! ## Example
//!
//! ```ignore
//! use angelscript_compiler::{CompilationContext, TypeResolver};
//!
//! let ctx = CompilationContext::new(&registry);
//! let resolver = TypeResolver::new(&ctx);
//!
//! // Resolve "const int@"
//! let data_type = resolver.resolve(&type_expr)?;
//! assert!(data_type.is_const);
//! assert!(data_type.is_handle);
//! ```

use angelscript_core::{CompilationError, DataType, RefModifier, TypeHash, primitives};
use angelscript_parser::ast::{PrimitiveType, RefKind, TypeBase, TypeExpr, TypeSuffix};

use crate::context::CompilationContext;

/// Resolves AST type expressions to semantic DataTypes.
///
/// Uses the compilation context's materialized scope for O(1) type name resolution.
pub struct TypeResolver<'a, 'reg> {
    ctx: &'a CompilationContext<'reg>,
}

impl<'a, 'reg> TypeResolver<'a, 'reg> {
    /// Create a new type resolver with the given compilation context.
    pub fn new(ctx: &'a CompilationContext<'reg>) -> Self {
        Self { ctx }
    }

    /// Resolve a TypeExpr to a DataType.
    ///
    /// This handles:
    /// - Primitive types (void, int, float, etc.)
    /// - Named types (resolved via context)
    /// - Scoped types (Namespace::Type)
    /// - Type modifiers (const, handle, handle-to-const)
    ///
    /// Template arguments are not yet supported (Task 35).
    pub fn resolve(&self, type_expr: &TypeExpr<'_>) -> Result<DataType, CompilationError> {
        // Resolve the base type first
        let base_hash = self.resolve_base(type_expr)?;

        // Start with a simple type
        let mut data_type = DataType::simple(base_hash);

        // Apply leading const (makes the object const)
        if type_expr.is_const {
            data_type.is_const = true;
        }

        // Apply suffixes (handles)
        for suffix in type_expr.suffixes {
            match suffix {
                TypeSuffix::Handle { is_const } => {
                    data_type.is_handle = true;
                    if *is_const {
                        // Trailing const on handle (@ const) means handle-to-const
                        data_type.is_handle_to_const = true;
                    }
                }
            }
        }

        Ok(data_type)
    }

    /// Resolve a ParamType to a DataType (includes reference modifiers).
    ///
    /// This is used for function parameters where reference modifiers (&in, &out, &inout)
    /// are allowed.
    pub fn resolve_param(
        &self,
        param_type: &angelscript_parser::ast::ParamType<'_>,
    ) -> Result<DataType, CompilationError> {
        let mut data_type = self.resolve(&param_type.ty)?;

        // Apply reference modifier
        data_type.ref_modifier = match param_type.ref_kind {
            RefKind::None => RefModifier::None,
            RefKind::Ref => RefModifier::InOut, // Plain & is inout by default
            RefKind::RefIn => RefModifier::In,
            RefKind::RefOut => RefModifier::Out,
            RefKind::RefInOut => RefModifier::InOut,
        };

        Ok(data_type)
    }

    /// Resolve the base type hash from a TypeExpr.
    fn resolve_base(&self, type_expr: &TypeExpr<'_>) -> Result<TypeHash, CompilationError> {
        // Check for template arguments (not yet supported)
        if !type_expr.template_args.is_empty() {
            return Err(CompilationError::Other {
                message: "template instantiation not yet supported (Task 35)".to_string(),
                span: type_expr.span,
            });
        }

        match &type_expr.base {
            TypeBase::Primitive(prim) => Ok(self.primitive_to_hash(*prim)),

            TypeBase::Named(ident) => {
                // Check if there's a scope prefix
                if let Some(scope) = &type_expr.scope {
                    if scope.is_absolute {
                        // Absolute scope (::Type or ::Namespace::Type)
                        // Build qualified name from segments only, lookup bypasses current namespace
                        let qualified = self.build_absolute_name(scope, ident.name);
                        self.resolve_absolute_type(&qualified, type_expr.span)
                    } else {
                        // Relative qualified name (Namespace::Type)
                        let qualified = self.build_qualified_name(scope, ident.name);
                        self.ctx
                            .resolve_type(&qualified)
                            .ok_or(CompilationError::UnknownType {
                                name: qualified,
                                span: type_expr.span,
                            })
                    }
                } else {
                    // Simple unqualified name - use scope lookup
                    self.ctx
                        .resolve_type(ident.name)
                        .ok_or_else(|| CompilationError::UnknownType {
                            name: ident.name.to_string(),
                            span: type_expr.span,
                        })
                }
            }

            TypeBase::TemplateParam(_ident) => {
                // Template parameters are placeholders that get substituted during instantiation
                Err(CompilationError::Other {
                    message: "template parameter cannot be used as concrete type".to_string(),
                    span: type_expr.span,
                })
            }

            TypeBase::Auto => {
                // Auto type is resolved during type inference, not here
                // Return a special marker or error depending on context
                Err(CompilationError::Other {
                    message: "auto type cannot be resolved without inference context".to_string(),
                    span: type_expr.span,
                })
            }

            TypeBase::Unknown => Err(CompilationError::Other {
                message: "unknown type placeholder cannot be resolved".to_string(),
                span: type_expr.span,
            }),
        }
    }

    /// Build a qualified name from scope and identifier (for relative paths).
    fn build_qualified_name(
        &self,
        scope: &angelscript_parser::ast::Scope<'_>,
        name: &str,
    ) -> String {
        let mut parts: Vec<&str> = scope.segments.iter().map(|s| s.name).collect();
        parts.push(name);
        parts.join("::")
    }

    /// Build an absolute name from scope and identifier (for ::Type paths).
    /// This ignores the current namespace context.
    fn build_absolute_name(
        &self,
        scope: &angelscript_parser::ast::Scope<'_>,
        name: &str,
    ) -> String {
        if scope.segments.is_empty() {
            // ::Type - just the type name in global namespace
            name.to_string()
        } else {
            // ::Namespace::Type - qualified path from root
            let mut parts: Vec<&str> = scope.segments.iter().map(|s| s.name).collect();
            parts.push(name);
            parts.join("::")
        }
    }

    /// Resolve a type from an absolute path (::Type or ::Namespace::Type).
    /// This bypasses the current namespace scope and looks up directly.
    fn resolve_absolute_type(
        &self,
        name: &str,
        span: angelscript_core::Span,
    ) -> Result<TypeHash, CompilationError> {
        // For absolute paths, compute the hash and check registries directly
        let hash = TypeHash::from_name(name);

        // Check unit registry first, then global
        if self.ctx.get_type(hash).is_some() {
            Ok(hash)
        } else {
            Err(CompilationError::UnknownType {
                name: format!("::{}", name),
                span,
            })
        }
    }

    /// Map a primitive type to its TypeHash.
    fn primitive_to_hash(&self, prim: PrimitiveType) -> TypeHash {
        match prim {
            PrimitiveType::Void => primitives::VOID,
            PrimitiveType::Bool => primitives::BOOL,
            PrimitiveType::Int => primitives::INT32,
            PrimitiveType::Int8 => primitives::INT8,
            PrimitiveType::Int16 => primitives::INT16,
            PrimitiveType::Int64 => primitives::INT64,
            PrimitiveType::UInt => primitives::UINT32,
            PrimitiveType::UInt8 => primitives::UINT8,
            PrimitiveType::UInt16 => primitives::UINT16,
            PrimitiveType::UInt64 => primitives::UINT64,
            PrimitiveType::Float => primitives::FLOAT,
            PrimitiveType::Double => primitives::DOUBLE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{ClassEntry, Span, TypeKind};
    use angelscript_parser::ast::{Ident, ParamType};
    use angelscript_registry::SymbolRegistry;

    fn make_primitive_type(prim: PrimitiveType) -> TypeExpr<'static> {
        TypeExpr::primitive(prim, Span::new(1, 1, 3))
    }

    #[test]
    fn resolve_void() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Void);
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, primitives::VOID);
        assert!(!result.is_const);
        assert!(!result.is_handle);
    }

    #[test]
    fn resolve_int() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
    }

    #[test]
    fn resolve_all_primitives() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let cases = [
            (PrimitiveType::Void, primitives::VOID),
            (PrimitiveType::Bool, primitives::BOOL),
            (PrimitiveType::Int, primitives::INT32),
            (PrimitiveType::Int8, primitives::INT8),
            (PrimitiveType::Int16, primitives::INT16),
            (PrimitiveType::Int64, primitives::INT64),
            (PrimitiveType::UInt, primitives::UINT32),
            (PrimitiveType::UInt8, primitives::UINT8),
            (PrimitiveType::UInt16, primitives::UINT16),
            (PrimitiveType::UInt64, primitives::UINT64),
            (PrimitiveType::Float, primitives::FLOAT),
            (PrimitiveType::Double, primitives::DOUBLE),
        ];

        for (prim, expected_hash) in cases {
            let type_expr = make_primitive_type(prim);
            let result = resolver.resolve(&type_expr).unwrap();
            assert_eq!(
                result.type_hash, expected_hash,
                "primitive {:?} should map to correct hash",
                prim
            );
        }
    }

    #[test]
    fn resolve_const_int() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let mut type_expr = make_primitive_type(PrimitiveType::Int);
        type_expr.is_const = true;

        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert!(result.is_const);
        assert!(!result.is_handle);
    }

    #[test]
    fn resolve_named_type() {
        let mut registry = SymbolRegistry::with_primitives();

        // Register a class in the global namespace
        let class = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            TypeHash::from_name("Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = TypeExpr::named(Ident::new("Player", Span::new(1, 1, 6)));
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, TypeHash::from_name("Player"));
        assert!(!result.is_const);
        assert!(!result.is_handle);
    }

    #[test]
    fn resolve_unknown_type_error() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = TypeExpr::named(Ident::new("Unknown", Span::new(1, 1, 7)));
        let result = resolver.resolve(&type_expr);

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::UnknownType { name, .. } => {
                assert_eq!(name, "Unknown");
            }
            other => panic!("expected UnknownType error, got {:?}", other),
        }
    }

    #[test]
    fn resolve_handle_type() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();
        let class = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            TypeHash::from_name("Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let arena = Bump::new();
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let type_expr = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Player", Span::new(1, 1, 6))),
            &[],
            suffixes,
            Span::new(1, 1, 7),
        );

        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, TypeHash::from_name("Player"));
        assert!(!result.is_const);
        assert!(result.is_handle);
        assert!(!result.is_handle_to_const);
    }

    #[test]
    fn resolve_const_handle_type() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();
        let class = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            TypeHash::from_name("Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let arena = Bump::new();
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: true }]);
        // const Player@ const - leading const + trailing const on handle
        let type_expr = TypeExpr::new(
            true, // leading const
            None,
            TypeBase::Named(Ident::new("Player", Span::new(1, 7, 6))),
            &[],
            suffixes,
            Span::new(1, 1, 20),
        );

        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, TypeHash::from_name("Player"));
        assert!(result.is_const); // leading const
        assert!(result.is_handle);
        assert!(result.is_handle_to_const); // trailing const on handle
    }

    #[test]
    fn resolve_handle_to_const() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();
        let class = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            TypeHash::from_name("Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let arena = Bump::new();
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: true }]);
        // Player@ const - trailing const only (handle to const object)
        let type_expr = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Player", Span::new(1, 1, 6))),
            &[],
            suffixes,
            Span::new(1, 1, 13),
        );

        let result = resolver.resolve(&type_expr).unwrap();

        assert!(!result.is_const); // no leading const
        assert!(result.is_handle);
        assert!(result.is_handle_to_const); // trailing const
    }

    #[test]
    fn resolve_qualified_type() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();

        // Register a class in a namespace
        let class = ClassEntry::new(
            "Player",
            vec!["Game".to_string()],
            "Game::Player",
            TypeHash::from_name("Game::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let arena = Bump::new();
        let segments = arena.alloc_slice_copy(&[Ident::new("Game", Span::new(1, 1, 4))]);
        let scope = angelscript_parser::ast::Scope::new(false, segments, Span::new(1, 1, 4));

        let type_expr = TypeExpr::new(
            false,
            Some(scope),
            TypeBase::Named(Ident::new("Player", Span::new(1, 7, 6))),
            &[],
            &[],
            Span::new(1, 1, 12),
        );

        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, TypeHash::from_name("Game::Player"));
    }

    #[test]
    fn resolve_param_with_ref_in() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::RefIn, Span::new(1, 1, 7));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::In);
    }

    #[test]
    fn resolve_param_with_ref_out() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::RefOut, Span::new(1, 1, 8));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::Out);
    }

    #[test]
    fn resolve_param_with_ref_inout() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::RefInOut, Span::new(1, 1, 10));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::InOut);
    }

    #[test]
    fn resolve_param_plain_ref() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::Ref, Span::new(1, 1, 4));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        // Plain & defaults to InOut
        assert_eq!(result.ref_modifier, RefModifier::InOut);
    }

    #[test]
    fn resolve_param_by_value() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::None, Span::new(1, 1, 3));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::None);
    }

    #[test]
    fn resolve_auto_type_error() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = TypeExpr::new(false, None, TypeBase::Auto, &[], &[], Span::new(1, 1, 4));

        let result = resolver.resolve(&type_expr);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_unknown_base_error() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        let type_expr = TypeExpr::new(false, None, TypeBase::Unknown, &[], &[], Span::new(1, 1, 1));

        let result = resolver.resolve(&type_expr);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_with_namespace_context() {
        let mut registry = SymbolRegistry::with_primitives();

        // Register a class in Game namespace
        let class = ClassEntry::new(
            "Entity",
            vec!["Game".to_string()],
            "Game::Entity",
            TypeHash::from_name("Game::Entity"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Enter the Game namespace
        ctx.enter_namespace("Game");

        let resolver = TypeResolver::new(&ctx);

        // Should resolve unqualified "Entity" to Game::Entity
        let type_expr = TypeExpr::named(Ident::new("Entity", Span::new(1, 1, 6)));
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, TypeHash::from_name("Game::Entity"));
    }

    #[test]
    fn resolve_absolute_global_type() {
        let mut registry = SymbolRegistry::with_primitives();

        // Register a class in global namespace
        let global_class = ClassEntry::new(
            "Helper",
            vec![],
            "Helper",
            TypeHash::from_name("Helper"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        // Register a class in Game namespace with same name
        let game_class = ClassEntry::new(
            "Helper",
            vec!["Game".to_string()],
            "Game::Helper",
            TypeHash::from_name("Game::Helper"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(global_class.into()).unwrap();
        registry.register_type(game_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Enter the Game namespace
        ctx.enter_namespace("Game");

        let resolver = TypeResolver::new(&ctx);

        // Without ::, "Helper" should resolve to Game::Helper (current namespace)
        let unqualified = TypeExpr::named(Ident::new("Helper", Span::new(1, 1, 6)));
        let result = resolver.resolve(&unqualified).unwrap();
        assert_eq!(result.type_hash, TypeHash::from_name("Game::Helper"));

        // With ::Helper, should resolve to global Helper
        let scope = angelscript_parser::ast::Scope::new(
            true, // is_absolute
            &[],  // no segments - just ::Helper
            Span::new(1, 1, 2),
        );
        let absolute = TypeExpr::new(
            false,
            Some(scope),
            TypeBase::Named(Ident::new("Helper", Span::new(1, 3, 6))),
            &[],
            &[],
            Span::new(1, 1, 8),
        );
        let result = resolver.resolve(&absolute).unwrap();
        assert_eq!(result.type_hash, TypeHash::from_name("Helper"));
    }

    #[test]
    fn resolve_absolute_namespaced_type() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();

        // Register a class in Utils namespace
        let utils_class = ClassEntry::new(
            "Logger",
            vec!["Utils".to_string()],
            "Utils::Logger",
            TypeHash::from_name("Utils::Logger"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(utils_class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Enter some other namespace
        ctx.enter_namespace("Game");

        let resolver = TypeResolver::new(&ctx);

        let arena = Bump::new();
        // ::Utils::Logger - absolute path to Utils::Logger
        let segments = arena.alloc_slice_copy(&[Ident::new("Utils", Span::new(1, 3, 5))]);
        let scope = angelscript_parser::ast::Scope::new(
            true, // is_absolute
            segments,
            Span::new(1, 1, 7),
        );
        let absolute = TypeExpr::new(
            false,
            Some(scope),
            TypeBase::Named(Ident::new("Logger", Span::new(1, 10, 6))),
            &[],
            &[],
            Span::new(1, 1, 15),
        );
        let result = resolver.resolve(&absolute).unwrap();
        assert_eq!(result.type_hash, TypeHash::from_name("Utils::Logger"));
    }

    #[test]
    fn resolve_absolute_unknown_type_error() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);
        let resolver = TypeResolver::new(&ctx);

        // ::Unknown - absolute path to non-existent type
        let scope = angelscript_parser::ast::Scope::new(true, &[], Span::new(1, 1, 2));
        let absolute = TypeExpr::new(
            false,
            Some(scope),
            TypeBase::Named(Ident::new("Unknown", Span::new(1, 3, 7))),
            &[],
            &[],
            Span::new(1, 1, 9),
        );
        let result = resolver.resolve(&absolute);
        assert!(result.is_err());

        match result.unwrap_err() {
            CompilationError::UnknownType { name, .. } => {
                assert_eq!(name, "::Unknown");
            }
            other => panic!("expected UnknownType error, got {:?}", other),
        }
    }
}
