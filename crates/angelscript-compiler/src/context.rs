//! CompilationContext - unified context for compilation.

use crate::registry::ScriptRegistry;

/// Unified compilation context with registry and name resolution.
pub struct CompilationContext {
    script: ScriptRegistry,
    namespace_path: Vec<String>,
    imported_namespaces: Vec<String>,
}

impl CompilationContext {
    pub fn new() -> Self {
        Self {
            script: ScriptRegistry::new(),
            namespace_path: Vec::new(),
            imported_namespaces: Vec::new(),
        }
    }
}

impl Default for CompilationContext {
    fn default() -> Self {
        Self::new()
    }
}
