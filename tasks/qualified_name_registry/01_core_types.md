# Phase 1: Core Types

## Overview

Add foundational types to `angelscript-core` that enable deferred type resolution. These types form the intermediate representation produced by Pass 1.

**Files:**
- `crates/angelscript-core/src/qualified_name.rs` (new)
- `crates/angelscript-core/src/unresolved.rs` (new)
- `crates/angelscript-core/src/lib.rs` (update exports)

---

## QualifiedName

The primary identifier for types and functions during compilation. Used for name-based lookup before TypeHash is computed.

```rust
// crates/angelscript-core/src/qualified_name.rs

use std::fmt;

/// Qualified name for type/function identity during compilation.
///
/// Used as primary key for name resolution. TypeHash computed later for bytecode.
///
/// # Examples
///
/// ```
/// use angelscript_core::QualifiedName;
///
/// // Global namespace
/// let player = QualifiedName::global("Player");
/// assert_eq!(player.to_string(), "Player");
///
/// // With namespace
/// let entity = QualifiedName::new("Entity", vec!["Game".into(), "Core".into()]);
/// assert_eq!(entity.to_string(), "Game::Core::Entity");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    /// Simple name (e.g., "Player", "update")
    pub name: String,
    /// Namespace path (e.g., ["Game", "Entities"])
    /// Empty for global namespace
    pub namespace: Vec<String>,
}

impl QualifiedName {
    /// Create a new qualified name with namespace.
    pub fn new(name: impl Into<String>, namespace: Vec<String>) -> Self {
        Self {
            name: name.into(),
            namespace,
        }
    }

    /// Create a qualified name in the global namespace.
    pub fn global(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: Vec::new(),
        }
    }

    /// Create from a qualified string (e.g., "Game::Player").
    ///
    /// Splits on "::" - the last segment is the name, rest is namespace.
    pub fn from_qualified_string(s: &str) -> Self {
        let parts: Vec<&str> = s.split("::").collect();
        if parts.len() == 1 {
            Self::global(parts[0])
        } else {
            let name = parts.last().unwrap().to_string();
            let namespace = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            Self { name, namespace }
        }
    }

    /// Check if this is in the global namespace.
    pub fn is_global(&self) -> bool {
        self.namespace.is_empty()
    }

    /// Get the simple (unqualified) name.
    pub fn simple_name(&self) -> &str {
        &self.name
    }

    /// Get the namespace path.
    pub fn namespace_path(&self) -> &[String] {
        &self.namespace
    }

    /// Get the namespace as a joined string.
    pub fn namespace_string(&self) -> String {
        self.namespace.join("::")
    }

    /// Compute TypeHash from this qualified name.
    ///
    /// Note: This is relatively expensive. Cache the result if called repeatedly.
    pub fn to_type_hash(&self) -> crate::TypeHash {
        crate::TypeHash::from_name(&self.to_string())
    }

    /// Create a child name within this namespace.
    ///
    /// Example: `Game::Core` + `Player` = `Game::Core::Player`
    pub fn child(&self, name: impl Into<String>) -> Self {
        let mut child_ns = self.namespace.clone();
        child_ns.push(self.name.clone());
        Self {
            name: name.into(),
            namespace: child_ns,
        }
    }

    /// Get the parent namespace as a QualifiedName (if any).
    ///
    /// Example: `Game::Core::Player` -> Some(`Game::Core`)
    pub fn parent(&self) -> Option<Self> {
        if self.namespace.is_empty() {
            None
        } else {
            let name = self.namespace.last().unwrap().clone();
            let namespace = self.namespace[..self.namespace.len() - 1].to_vec();
            Some(Self { name, namespace })
        }
    }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.namespace.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}::{}", self.namespace.join("::"), self.name)
        }
    }
}

impl From<&str> for QualifiedName {
    fn from(s: &str) -> Self {
        Self::from_qualified_string(s)
    }
}

impl From<String> for QualifiedName {
    fn from(s: String) -> Self {
        Self::from_qualified_string(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_name() {
        let name = QualifiedName::global("Player");
        assert_eq!(name.name, "Player");
        assert!(name.namespace.is_empty());
        assert!(name.is_global());
        assert_eq!(name.to_string(), "Player");
    }

    #[test]
    fn namespaced_name() {
        let name = QualifiedName::new("Player", vec!["Game".into(), "Entities".into()]);
        assert_eq!(name.name, "Player");
        assert_eq!(name.namespace, vec!["Game", "Entities"]);
        assert!(!name.is_global());
        assert_eq!(name.to_string(), "Game::Entities::Player");
    }

    #[test]
    fn from_qualified_string() {
        let name = QualifiedName::from_qualified_string("Game::Entities::Player");
        assert_eq!(name.name, "Player");
        assert_eq!(name.namespace, vec!["Game", "Entities"]);

        let global = QualifiedName::from_qualified_string("int");
        assert_eq!(global.name, "int");
        assert!(global.namespace.is_empty());
    }

    #[test]
    fn child_name() {
        let parent = QualifiedName::new("Core", vec!["Game".into()]);
        let child = parent.child("Player");
        assert_eq!(child.to_string(), "Game::Core::Player");
    }

    #[test]
    fn parent_name() {
        let name = QualifiedName::new("Player", vec!["Game".into(), "Core".into()]);
        let parent = name.parent().unwrap();
        assert_eq!(parent.to_string(), "Game::Core");

        let global = QualifiedName::global("int");
        assert!(global.parent().is_none());
    }

    #[test]
    fn hash_equality() {
        use std::collections::HashSet;

        let a = QualifiedName::new("Player", vec!["Game".into()]);
        let b = QualifiedName::new("Player", vec!["Game".into()]);
        let c = QualifiedName::new("Enemy", vec!["Game".into()]);

        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a.clone());
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }
}
```

---

## UnresolvedType

Captures a type reference as written in source, to be resolved later in Pass 2.

```rust
// crates/angelscript-core/src/unresolved.rs

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
        let param = UnresolvedParam::new(
            "target",
            UnresolvedType::simple("Player").with_handle(true),
        );

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
```

---

## Module Exports

Update `lib.rs` to export the new types:

```rust
// In crates/angelscript-core/src/lib.rs

mod qualified_name;
mod unresolved;

pub use qualified_name::QualifiedName;
pub use unresolved::{UnresolvedParam, UnresolvedSignature, UnresolvedType};
```

---

## Dependencies

These types have minimal dependencies:
- `QualifiedName` only depends on `TypeHash` for the `to_type_hash()` method
- `UnresolvedType` only depends on `RefModifier`

This makes them safe to add without circular dependency issues.

---

## What's Next

Phase 2 will define `UnresolvedClass`, `UnresolvedInterface`, etc. that use these core types to represent complete type declarations from Pass 1.
