use crate::RefModifier;

/// Unresolved type reference - stored during registration, resolved in completion.
///
/// This captures exactly what was written in the source code, plus the context
/// needed to resolve it later (namespace and imports).
///
/// # Examples
///
/// For `const Player@ &in`:
/// ```ignore
/// UnresolvedType {
///     name: "Player",
///     context_namespace: vec!["Game", "Entities"],
///     imports: vec!["Utils"],
///     is_const: true,
///     is_handle: true,
///     is_handle_to_const: false,
///     ref_modifier: RefModifier::In,
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnresolvedType {
    /// The type name as written (e.g., "Player", "Game::Entity", "array<int>")
    pub name: String,

    /// Namespace context where this reference appeared.
    /// Used for relative name resolution.
    pub context_namespace: Vec<String>,

    /// Imports active when this reference appeared.
    /// Each import is tried as a prefix during resolution.
    pub imports: Vec<String>,

    /// Leading `const` modifier.
    pub is_const: bool,

    /// Handle (`@`) modifier.
    pub is_handle: bool,

    /// Handle-to-const (`const@` or `@const`) modifier.
    pub is_handle_to_const: bool,

    /// Reference modifier for parameters (`&in`, `&out`, `&inout`).
    pub ref_modifier: RefModifier,
}

impl UnresolvedType {
    /// Create a simple unresolved type (no modifiers, global context).
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Create with namespace context.
    pub fn with_context(
        name: impl Into<String>,
        context_namespace: Vec<String>,
        imports: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            context_namespace,
            imports,
            ..Default::default()
        }
    }

    /// Set const modifier.
    pub fn with_const(mut self, is_const: bool) -> Self {
        self.is_const = is_const;
        self
    }

    /// Set handle modifier.
    pub fn with_handle(mut self, is_handle: bool) -> Self {
        self.is_handle = is_handle;
        self
    }

    /// Set handle-to-const modifier.
    pub fn with_handle_to_const(mut self, is_handle_to_const: bool) -> Self {
        self.is_handle_to_const = is_handle_to_const;
        self
    }

    /// Set reference modifier.
    pub fn with_ref_modifier(mut self, ref_modifier: RefModifier) -> Self {
        self.ref_modifier = ref_modifier;
        self
    }

    /// Check if this is a void type.
    pub fn is_void(&self) -> bool {
        self.name == "void"
    }

    /// Check if the name contains namespace qualifiers.
    pub fn is_qualified(&self) -> bool {
        self.name.contains("::")
    }

    /// Check if this is a template type (contains `<`).
    pub fn is_template(&self) -> bool {
        self.name.contains('<')
    }
}

/// Parameter with unresolved type (during registration).
#[derive(Debug, Clone, PartialEq)]
pub struct UnresolvedParam {
    /// Parameter name (may be empty for unnamed params).
    pub name: String,

    /// The unresolved parameter type.
    pub param_type: UnresolvedType,

    /// Whether this parameter has a default value.
    /// The actual default is not stored here - it's compiled later.
    pub has_default: bool,
}

impl UnresolvedParam {
    /// Create a new unresolved parameter.
    pub fn new(name: impl Into<String>, param_type: UnresolvedType) -> Self {
        Self {
            name: name.into(),
            param_type,
            has_default: false,
        }
    }

    /// Mark as having a default value.
    pub fn with_default(mut self) -> Self {
        self.has_default = true;
        self
    }
}

/// Unresolved function signature (during registration).
///
/// Contains the raw type names that will be resolved in the completion pass.
#[derive(Debug, Clone, PartialEq)]
pub struct UnresolvedSignature {
    /// Function/method name.
    pub name: String,

    /// Unresolved parameter types.
    pub params: Vec<UnresolvedParam>,

    /// Unresolved return type.
    pub return_type: UnresolvedType,

    /// Whether this is a const method.
    pub is_const: bool,
}

impl UnresolvedSignature {
    /// Create a new unresolved signature.
    pub fn new(
        name: impl Into<String>,
        params: Vec<UnresolvedParam>,
        return_type: UnresolvedType,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
            is_const: false,
        }
    }

    /// Set const flag.
    pub fn with_const(mut self, is_const: bool) -> Self {
        self.is_const = is_const;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_unresolved_type() {
        let ty = UnresolvedType::simple("int");
        assert_eq!(ty.name, "int");
        assert!(!ty.is_const);
        assert!(!ty.is_handle);
        assert!(!ty.is_qualified());
    }

    #[test]
    fn unresolved_type_with_modifiers() {
        let ty = UnresolvedType::simple("Player")
            .with_const(true)
            .with_handle(true)
            .with_ref_modifier(RefModifier::In);

        assert!(ty.is_const);
        assert!(ty.is_handle);
        assert_eq!(ty.ref_modifier, RefModifier::In);
    }

    #[test]
    fn qualified_unresolved_type() {
        let ty = UnresolvedType::simple("Game::Player");
        assert!(ty.is_qualified());
    }

    #[test]
    fn template_unresolved_type() {
        let ty = UnresolvedType::simple("array<int>");
        assert!(ty.is_template());
        assert!(!ty.is_qualified());
    }

    #[test]
    fn unresolved_param() {
        let param =
            UnresolvedParam::new("target", UnresolvedType::simple("Player").with_handle(true));

        assert_eq!(param.name, "target");
        assert!(param.param_type.is_handle);
        assert!(!param.has_default);
    }

    #[test]
    fn unresolved_param_with_default() {
        let param = UnresolvedParam::new("count", UnresolvedType::simple("int")).with_default();
        assert!(param.has_default);
    }

    #[test]
    fn unresolved_signature() {
        let sig = UnresolvedSignature::new(
            "attack",
            vec![UnresolvedParam::new(
                "target",
                UnresolvedType::simple("Enemy").with_handle(true),
            )],
            UnresolvedType::simple("void"),
        )
        .with_const(false);

        assert_eq!(sig.name, "attack");
        assert_eq!(sig.params.len(), 1);
        assert!(sig.return_type.is_void());
        assert!(!sig.is_const);
    }
}
