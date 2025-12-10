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
//! let mut ctx = CompilationContext::new(&registry);
//! let mut resolver = TypeResolver::new(&mut ctx);
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
/// Template instantiation is performed when resolving types with template arguments.
pub struct TypeResolver<'a, 'reg> {
    ctx: &'a mut CompilationContext<'reg>,
}

impl<'a, 'reg> TypeResolver<'a, 'reg> {
    /// Create a new type resolver with the given compilation context.
    pub fn new(ctx: &'a mut CompilationContext<'reg>) -> Self {
        Self { ctx }
    }

    /// Resolve a TypeExpr to a DataType.
    ///
    /// This handles:
    /// - Primitive types (void, int, float, etc.)
    /// - Named types (resolved via context)
    /// - Scoped types (Namespace::Type)
    /// - Type modifiers (const, handle, handle-to-const)
    /// - Template instantiation (e.g., `array<int>`)
    pub fn resolve(&mut self, type_expr: &TypeExpr<'_>) -> Result<DataType, CompilationError> {
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
        &mut self,
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
    fn resolve_base(&mut self, type_expr: &TypeExpr<'_>) -> Result<TypeHash, CompilationError> {
        // First resolve the base type name (without template args)
        let base_hash = self.resolve_base_name(type_expr)?;

        // If there are template arguments, instantiate the template
        if !type_expr.template_args.is_empty() {
            // Resolve each template argument type (recursive - handles nested templates)
            let mut resolved_args = Vec::with_capacity(type_expr.template_args.len());
            for arg in type_expr.template_args {
                let arg_type = self.resolve(arg)?;
                resolved_args.push(arg_type);
            }

            // Instantiate the template with the resolved arguments
            self.ctx
                .instantiate_template(base_hash, &resolved_args, type_expr.span)
        } else {
            Ok(base_hash)
        }
    }

    /// Resolve just the base type name (without template arguments).
    fn resolve_base_name(&self, type_expr: &TypeExpr<'_>) -> Result<TypeHash, CompilationError> {
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
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = make_primitive_type(PrimitiveType::Void);
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, primitives::VOID);
        assert!(!result.is_const);
        assert!(!result.is_handle);
    }

    #[test]
    fn resolve_int() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
    }

    #[test]
    fn resolve_all_primitives() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = TypeExpr::named(Ident::new("Player", Span::new(1, 1, 6)));
        let result = resolver.resolve(&type_expr).unwrap();

        assert_eq!(result.type_hash, TypeHash::from_name("Player"));
        assert!(!result.is_const);
        assert!(!result.is_handle);
    }

    #[test]
    fn resolve_unknown_type_error() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::RefIn, Span::new(1, 1, 7));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::In);
    }

    #[test]
    fn resolve_param_with_ref_out() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::RefOut, Span::new(1, 1, 8));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::Out);
    }

    #[test]
    fn resolve_param_with_ref_inout() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::RefInOut, Span::new(1, 1, 10));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::InOut);
    }

    #[test]
    fn resolve_param_plain_ref() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = make_primitive_type(PrimitiveType::Int);
        let param_type = ParamType::new(type_expr, RefKind::None, Span::new(1, 1, 3));

        let result = resolver.resolve_param(&param_type).unwrap();

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::None);
    }

    #[test]
    fn resolve_auto_type_error() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let type_expr = TypeExpr::new(false, None, TypeBase::Auto, &[], &[], Span::new(1, 1, 4));

        let result = resolver.resolve(&type_expr);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_unknown_base_error() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut resolver = TypeResolver::new(&mut ctx);

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

        let mut resolver = TypeResolver::new(&mut ctx);

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
        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

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

    // =========================================================================
    // Template Instantiation Tests
    // =========================================================================

    use angelscript_core::{
        FunctionDef, FunctionEntry, FunctionTraits, Param, TemplateParamEntry, Visibility,
        entries::TypeSource,
    };

    /// Helper to create an array template with a push method.
    fn create_array_template(registry: &mut SymbolRegistry) -> TypeHash {
        let array_hash = TypeHash::from_name("array");

        // Create template param T
        let t_param = TemplateParamEntry::for_template("T", 0, array_hash, "array");
        let t_hash = t_param.type_hash;
        registry.register_type(t_param.into()).unwrap();

        // Create array template
        let array_entry = ClassEntry::new(
            "array",
            vec![],
            "array",
            array_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_template_params(vec![t_hash]);

        registry.register_type(array_entry.into()).unwrap();

        // Create push method: void push(const T&in)
        let push_hash = TypeHash::from_method(array_hash, "push", &[t_hash]);
        let push_def = FunctionDef::new(
            push_hash,
            "push".to_string(),
            vec![],
            vec![Param::new("value", DataType::with_ref_in(t_hash))],
            DataType::void(),
            Some(array_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(push_def))
            .unwrap();

        // Add method to class
        if let Some(entry) = registry.get_mut(array_hash)
            && let Some(class) = entry.as_class_mut()
        {
            class.add_method("push", push_hash);
        }

        array_hash
    }

    #[test]
    fn resolve_simple_template_instantiation() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();
        create_array_template(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let arena = Bump::new();
        // Create array<int> type expression
        let int_type_expr = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 7, 3));
        let template_args = arena.alloc_slice_copy(&[int_type_expr]);

        let type_expr = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("array", Span::new(1, 1, 5))),
            template_args,
            &[],
            Span::new(1, 1, 11),
        );

        let result = resolver.resolve(&type_expr);
        assert!(result.is_ok(), "Failed to resolve array<int>: {:?}", result);

        let data_type = result.unwrap();

        // Verify the instance was created
        let instance = ctx.get_type(data_type.type_hash);
        assert!(instance.is_some(), "Instance should exist in registry");

        let class = instance.unwrap().as_class().unwrap();
        assert!(class.is_template_instance());
        assert_eq!(class.qualified_name, "array<int>");
    }

    #[test]
    fn resolve_nested_template_instantiation() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();
        create_array_template(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let arena = Bump::new();

        // Create array<array<int>> type expression
        // First, inner: array<int>
        let int_type_expr = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 13, 3));
        let inner_template_args = arena.alloc_slice_copy(&[int_type_expr]);
        let inner_array = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("array", Span::new(1, 7, 5))),
            inner_template_args,
            &[],
            Span::new(1, 7, 11),
        );

        // Outer: array<array<int>>
        let outer_template_args = arena.alloc_slice_copy(&[inner_array]);
        let outer_array = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("array", Span::new(1, 1, 5))),
            outer_template_args,
            &[],
            Span::new(1, 1, 18),
        );

        let result = resolver.resolve(&outer_array);
        assert!(
            result.is_ok(),
            "Failed to resolve array<array<int>>: {:?}",
            result
        );

        let outer_type = result.unwrap();

        // Verify the outer instance was created
        let outer_instance = ctx.get_type(outer_type.type_hash);
        assert!(outer_instance.is_some(), "Outer instance should exist");

        let outer_class = outer_instance.unwrap().as_class().unwrap();
        assert!(outer_class.is_template_instance());
        assert_eq!(outer_class.qualified_name, "array<array<int>>");

        // Verify the inner instance was also created
        let inner_hash =
            TypeHash::from_template_instance(TypeHash::from_name("array"), &[primitives::INT32]);
        let inner_instance = ctx.get_type(inner_hash);
        assert!(inner_instance.is_some(), "Inner instance should exist");

        let inner_class = inner_instance.unwrap().as_class().unwrap();
        assert!(inner_class.is_template_instance());
        assert_eq!(inner_class.qualified_name, "array<int>");
    }

    #[test]
    fn resolve_template_not_a_template_error() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();

        // Register a non-template class
        let player = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            TypeHash::from_name("Player"),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(player.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let arena = Bump::new();
        // Try to instantiate Player<int> - should fail
        let int_type_expr = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 8, 3));
        let template_args = arena.alloc_slice_copy(&[int_type_expr]);

        let type_expr = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Player", Span::new(1, 1, 6))),
            template_args,
            &[],
            Span::new(1, 1, 12),
        );

        let result = resolver.resolve(&type_expr);
        assert!(result.is_err());

        match result.unwrap_err() {
            CompilationError::NotATemplate { name, .. } => {
                assert_eq!(name, "Player");
            }
            other => panic!("expected NotATemplate error, got {:?}", other),
        }
    }

    #[test]
    fn resolve_template_caches_instances() {
        use bumpalo::Bump;

        let mut registry = SymbolRegistry::with_primitives();
        create_array_template(&mut registry);

        let mut ctx = CompilationContext::new(&registry);
        let mut resolver = TypeResolver::new(&mut ctx);

        let arena = Bump::new();

        // Create array<int> type expression
        let int_type_expr = TypeExpr::primitive(PrimitiveType::Int, Span::new(1, 7, 3));
        let template_args = arena.alloc_slice_copy(&[int_type_expr]);

        let type_expr = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("array", Span::new(1, 1, 5))),
            template_args,
            &[],
            Span::new(1, 1, 11),
        );

        // Resolve twice
        let result1 = resolver.resolve(&type_expr).unwrap();
        let result2 = resolver.resolve(&type_expr).unwrap();

        // Should return same hash (cached)
        assert_eq!(result1.type_hash, result2.type_hash);
    }
}
