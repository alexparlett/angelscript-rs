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
    /// Leading "::" (absolute path) is normalized: "::Game::Player" == "Game::Player".
    pub fn from_qualified_string(s: &str) -> Self {
        let parts: Vec<&str> = s.split("::").filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            // Empty string or just "::"
            Self::global("")
        } else if parts.len() == 1 {
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
    fn from_qualified_string_leading_colons() {
        // Leading :: (absolute path) should be normalized
        let absolute = QualifiedName::from_qualified_string("::Game::Player");
        let relative = QualifiedName::from_qualified_string("Game::Player");
        assert_eq!(absolute, relative);
        assert_eq!(absolute.name, "Player");
        assert_eq!(absolute.namespace, vec!["Game"]);

        // Global type with leading ::
        let global_absolute = QualifiedName::from_qualified_string("::int");
        let global_relative = QualifiedName::from_qualified_string("int");
        assert_eq!(global_absolute, global_relative);
        assert!(global_absolute.is_global());

        // Edge case: just "::"
        let empty = QualifiedName::from_qualified_string("::");
        assert_eq!(empty.name, "");
        assert!(empty.is_global());
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
