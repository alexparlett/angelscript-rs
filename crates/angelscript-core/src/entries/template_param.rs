//! Template parameter entry.
//!
//! This module provides `TemplateParamEntry` for template type parameters
//! like `T` in `array<T>`.

use crate::TypeHash;

/// Registry entry for a template type parameter.
///
/// Template parameters are placeholders like `T`, `K`, `V` in template
/// type definitions like `array<T>` or `dictionary<K, V>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParamEntry {
    /// Parameter name (e.g., "T", "K", "V").
    pub name: String,
    /// Parameter index within the template (0-based).
    pub index: usize,
    /// The template type hash this parameter belongs to.
    pub owner: TypeHash,
    /// Type hash for this parameter.
    pub type_hash: TypeHash,
}

impl TemplateParamEntry {
    /// Create a new template parameter entry.
    pub fn new(
        name: impl Into<String>,
        index: usize,
        owner: TypeHash,
        type_hash: TypeHash,
    ) -> Self {
        Self {
            name: name.into(),
            index,
            owner,
            type_hash,
        }
    }

    /// Create a template parameter with auto-generated hash.
    ///
    /// The hash is generated from the owner and parameter name.
    pub fn for_template(
        name: impl Into<String>,
        index: usize,
        owner: TypeHash,
        owner_name: &str,
    ) -> Self {
        let name = name.into();
        let qualified = format!("{}::{}", owner_name, name);
        Self {
            type_hash: TypeHash::from_name(&qualified),
            name,
            index,
            owner,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_param_creation() {
        let owner = TypeHash::from_name("array");
        let type_hash = TypeHash::from_name("array::T");
        let param = TemplateParamEntry::new("T", 0, owner, type_hash);

        assert_eq!(param.name, "T");
        assert_eq!(param.index, 0);
        assert_eq!(param.owner, owner);
        assert_eq!(param.type_hash, type_hash);
    }

    #[test]
    fn template_param_for_template() {
        let owner = TypeHash::from_name("dictionary");
        let param = TemplateParamEntry::for_template("K", 0, owner, "dictionary");

        assert_eq!(param.name, "K");
        assert_eq!(param.index, 0);
        assert_eq!(param.owner, owner);
        assert_eq!(param.type_hash, TypeHash::from_name("dictionary::K"));
    }

    #[test]
    fn multiple_template_params() {
        let owner = TypeHash::from_name("dictionary");
        let k = TemplateParamEntry::for_template("K", 0, owner, "dictionary");
        let v = TemplateParamEntry::for_template("V", 1, owner, "dictionary");

        assert_eq!(k.index, 0);
        assert_eq!(v.index, 1);
        assert_ne!(k.type_hash, v.type_hash);
    }
}
