//! Compilation context with namespace-aware symbol resolution.
//!
//! This module provides [`CompilationContext`], which wraps registries and provides
//! O(1) symbol resolution through a materialized [`Scope`] view.
//!
//! ## Design: Materialized Scope View
//!
//! Instead of O(m) iteration through namespaces on each lookup, we maintain a
//! `Scope` that is a materialized view of all accessible symbols. This is rebuilt
//! when namespace changes occur (enter/exit namespace, add import).
//!
//! **Complexity:**
//! - `resolve_type()`: O(1) - single HashMap lookup
//! - `enter_namespace()`: O(t) - rebuilds scope where t = total accessible types
//! - Namespace changes are infrequent, resolutions are frequent, so this is optimal.

use rustc_hash::FxHashMap;

use angelscript_core::{
    CompilationError, FunctionEntry, GlobalPropertyEntry, RegistrationError, Span, TypeEntry,
    TypeHash,
};
use angelscript_registry::SymbolRegistry;

// ============================================================================
// Scope
// ============================================================================

/// Materialized view of symbols accessible without qualification.
///
/// Rebuilt when namespace changes or imports are added.
/// Provides O(1) lookup for unqualified names.
#[derive(Debug, Default)]
pub struct Scope {
    /// Simple name -> TypeHash (e.g., "Player" -> hash of "Game::Entities::Player")
    pub types: FxHashMap<String, TypeHash>,

    /// Simple name -> function hashes (multiple for overloads)
    pub functions: FxHashMap<String, Vec<TypeHash>>,

    /// Simple name -> global property hash
    pub globals: FxHashMap<String, TypeHash>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all entries from the scope.
    pub fn clear(&mut self) {
        self.types.clear();
        self.functions.clear();
        self.globals.clear();
    }
}

// ============================================================================
// CompilationContext
// ============================================================================

/// Compilation context with layered registries and namespace-aware resolution.
///
/// Provides O(1) symbol resolution through a materialized scope view that is
/// rebuilt when namespace changes occur.
pub struct CompilationContext<'a> {
    /// Global registry (FFI types, shared types)
    global_registry: &'a SymbolRegistry,

    /// Unit-local registry (script types being compiled)
    unit_registry: SymbolRegistry,

    /// Materialized scope for O(1) resolution
    scope: Scope,

    /// Namespace stack for current position (e.g., ["Game", "Entities"])
    namespace_stack: Vec<String>,

    /// Active using namespace imports
    imports: Vec<String>,

    /// Errors collected during compilation
    errors: Vec<CompilationError>,
}

impl<'a> CompilationContext<'a> {
    /// Create a new compilation context with a reference to the global registry.
    pub fn new(global_registry: &'a SymbolRegistry) -> Self {
        let mut ctx = Self {
            global_registry,
            unit_registry: SymbolRegistry::new(),
            scope: Scope::new(),
            namespace_stack: Vec::new(),
            imports: Vec::new(),
            errors: Vec::new(),
        };
        // Build initial scope with global namespace
        ctx.rebuild_scope();
        ctx
    }

    // ========================================================================
    // Namespace Management
    // ========================================================================

    /// Enter a namespace block: `namespace Game::Entities { ... }`
    pub fn enter_namespace(&mut self, ns: &str) {
        self.namespace_stack.push(ns.to_string());
        self.rebuild_scope();
    }

    /// Exit a namespace block.
    pub fn exit_namespace(&mut self) {
        self.namespace_stack.pop();
        self.rebuild_scope();
    }

    /// Process: `using namespace Game::Utils;`
    pub fn add_import(&mut self, ns: &str) {
        if !self.imports.contains(&ns.to_string()) {
            self.imports.push(ns.to_string());
            self.rebuild_scope();
        }
    }

    /// Get current namespace as qualified string.
    pub fn current_namespace(&self) -> String {
        self.namespace_stack.join("::")
    }

    // ========================================================================
    // Scope Building (O(t) where t = total accessible types)
    // ========================================================================

    /// Rebuild the materialized scope from scratch.
    /// Called when namespace changes or imports are added.
    fn rebuild_scope(&mut self) {
        self.scope.clear();

        // Build order matters for shadowing:
        // 1. Global namespace (lowest priority)
        // 2. Imported namespaces
        // 3. Current namespace (highest priority - shadows imports)

        // 1. Add global namespace (always accessible)
        self.add_namespace_to_scope("");

        // 2. Add imported namespaces
        for import in self.imports.clone() {
            self.add_namespace_to_scope(&import);
        }

        // 3. Add current namespace (highest priority - shadows imports)
        let current = self.current_namespace();
        if !current.is_empty() {
            self.add_namespace_to_scope(&current);
        }
    }

    /// Add all symbols from a namespace to the current scope.
    fn add_namespace_to_scope(&mut self, ns: &str) {
        // Collect entries first to avoid borrow checker issues
        let mut type_entries: Vec<(String, TypeHash)> = Vec::new();
        let mut func_entries: Vec<(String, TypeHash)> = Vec::new();
        let mut global_entries: Vec<(String, TypeHash)> = Vec::new();

        // Collect types from unit registry
        if let Some(types) = self.unit_registry.get_namespace_types(ns) {
            for (simple, &hash) in types {
                type_entries.push((simple.clone(), hash));
            }
        }

        // Collect types from global registry
        if let Some(types) = self.global_registry.get_namespace_types(ns) {
            for (simple, &hash) in types {
                type_entries.push((simple.clone(), hash));
            }
        }

        // Collect functions from unit registry
        if let Some(funcs) = self.unit_registry.get_namespace_functions(ns) {
            for (simple, hashes) in funcs {
                for &hash in hashes {
                    func_entries.push((simple.clone(), hash));
                }
            }
        }

        // Collect functions from global registry
        if let Some(funcs) = self.global_registry.get_namespace_functions(ns) {
            for (simple, hashes) in funcs {
                for &hash in hashes {
                    func_entries.push((simple.clone(), hash));
                }
            }
        }

        // Collect globals from unit registry
        if let Some(globals) = self.unit_registry.get_namespace_globals(ns) {
            for (simple, &hash) in globals {
                global_entries.push((simple.clone(), hash));
            }
        }

        // Collect globals from global registry
        if let Some(globals) = self.global_registry.get_namespace_globals(ns) {
            for (simple, &hash) in globals {
                global_entries.push((simple.clone(), hash));
            }
        }

        // Now add to scope (no longer borrowing registries)
        for (simple, hash) in type_entries {
            self.add_type_to_scope(&simple, hash, ns);
        }

        for (simple, hash) in func_entries {
            self.add_function_to_scope(&simple, hash);
        }

        for (simple, hash) in global_entries {
            self.add_global_to_scope(&simple, hash, ns);
        }
    }

    fn add_type_to_scope(&mut self, simple: &str, hash: TypeHash, ns: &str) {
        if let Some(&existing) = self.scope.types.get(simple)
            && existing != hash
        {
            let current = self.current_namespace();

            // Determine if we should report an ambiguity error.
            // Shadowing rules (in order of priority):
            // 1. Current namespace shadows everything - NO error
            // 2. Import shadows global namespace - NO error
            // 3. Import conflicts with another import - ERROR (ambiguity)

            let is_from_current_ns = ns == current && !current.is_empty();
            let existing_is_from_global = self
                .get_type(existing)
                .map(|e| e.namespace().is_empty())
                .unwrap_or(false);

            // Only report ambiguity if:
            // - New type is NOT from current namespace, AND
            // - Existing type is NOT from global namespace (i.e., both are from imports)
            if !is_from_current_ns && !existing_is_from_global {
                let existing_name = self
                    .get_type(existing)
                    .map(|e| e.qualified_name().to_string())
                    .unwrap_or_else(|| format!("{:?}", existing));
                let new_name = self
                    .get_type(hash)
                    .map(|e| e.qualified_name().to_string())
                    .unwrap_or_else(|| format!("{:?}", hash));

                self.errors.push(CompilationError::AmbiguousSymbol {
                    kind: "type".to_string(),
                    name: simple.to_string(),
                    candidates: format!("{}, {}", existing_name, new_name),
                    span: Span::default(),
                });
            }
        }
        // Later additions (current namespace) shadow earlier ones (imports/global)
        self.scope.types.insert(simple.to_string(), hash);
    }

    fn add_function_to_scope(&mut self, simple: &str, hash: TypeHash) {
        // Functions can have multiple overloads - collect all
        let entry = self.scope.functions.entry(simple.to_string()).or_default();
        if !entry.contains(&hash) {
            entry.push(hash);
        }
    }

    fn add_global_to_scope(&mut self, simple: &str, hash: TypeHash, ns: &str) {
        if let Some(&existing) = self.scope.globals.get(simple)
            && existing != hash
        {
            let current = self.current_namespace();

            // Determine if we should report an ambiguity error.
            // Shadowing rules (in order of priority):
            // 1. Current namespace shadows everything - NO error
            // 2. Import shadows global namespace - NO error
            // 3. Import conflicts with another import - ERROR (ambiguity)

            let is_from_current_ns = ns == current && !current.is_empty();
            let existing_is_from_global = self
                .get_global_entry(existing)
                .map(|e| e.namespace.is_empty())
                .unwrap_or(false);

            // Only report ambiguity if:
            // - New global is NOT from current namespace, AND
            // - Existing global is NOT from global namespace (i.e., both are from imports)
            if !is_from_current_ns && !existing_is_from_global {
                let existing_name = self
                    .get_global_entry(existing)
                    .map(|e| e.qualified_name.clone())
                    .unwrap_or_else(|| format!("{:?}", existing));
                let new_name = self
                    .get_global_entry(hash)
                    .map(|e| e.qualified_name.clone())
                    .unwrap_or_else(|| format!("{:?}", hash));

                self.errors.push(CompilationError::AmbiguousSymbol {
                    kind: "global variable".to_string(),
                    name: simple.to_string(),
                    candidates: format!("{}, {}", existing_name, new_name),
                    span: Span::default(),
                });
            }
        }
        self.scope.globals.insert(simple.to_string(), hash);
    }

    // ========================================================================
    // Resolution Methods (O(1))
    // ========================================================================

    /// Resolve a type name to its hash. O(1) for unqualified, O(1) for qualified.
    pub fn resolve_type(&self, name: &str) -> Option<TypeHash> {
        if name.contains("::") {
            // Qualified name: bypass scope, direct registry lookup
            let hash = TypeHash::from_name(name);
            if self.unit_registry.get(hash).is_some() || self.global_registry.get(hash).is_some() {
                return Some(hash);
            }
            return None;
        }

        // Unqualified: single scope lookup - O(1)
        self.scope.types.get(name).copied()
    }

    /// Resolve a function name to all matching overloads. O(1).
    pub fn resolve_function(&self, name: &str) -> Option<&[TypeHash]> {
        if name.contains("::") {
            // Qualified function lookup would need additional handling
            return None;
        }

        self.scope.functions.get(name).map(|v| v.as_slice())
    }

    /// Resolve a global variable name to its hash. O(1).
    pub fn resolve_global(&self, name: &str) -> Option<TypeHash> {
        if name.contains("::") {
            let hash = TypeHash::from_name(name);
            if self.unit_registry.get_global(hash).is_some()
                || self.global_registry.get_global(hash).is_some()
            {
                return Some(hash);
            }
            return None;
        }

        self.scope.globals.get(name).copied()
    }

    // ========================================================================
    // Direct Registry Access (by hash) - for after resolution
    // ========================================================================

    /// Get a type entry by hash (layered lookup).
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.unit_registry
            .get(hash)
            .or_else(|| self.global_registry.get(hash))
    }

    /// Get a function entry by hash (layered lookup).
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.unit_registry
            .get_function(hash)
            .or_else(|| self.global_registry.get_function(hash))
    }

    /// Get a global entry by hash (layered lookup).
    pub fn get_global_entry(&self, hash: TypeHash) -> Option<&GlobalPropertyEntry> {
        self.unit_registry
            .get_global(hash)
            .or_else(|| self.global_registry.get_global(hash))
    }

    /// Find methods on a type by name.
    pub fn find_methods(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        let mut methods = Vec::new();

        // Check type in unit registry
        if let Some(class) = self.unit_registry.get(type_hash).and_then(|e| e.as_class()) {
            for &method_hash in &class.methods {
                if let Some(func) = self.get_function(method_hash)
                    && func.def.name == name
                {
                    methods.push(method_hash);
                }
            }
        }

        // Check type in global registry
        if let Some(class) = self
            .global_registry
            .get(type_hash)
            .and_then(|e| e.as_class())
        {
            for &method_hash in &class.methods {
                if let Some(func) = self.get_function(method_hash)
                    && func.def.name == name
                {
                    methods.push(method_hash);
                }
            }
        }

        methods
    }

    // ========================================================================
    // Registration (for unit registry)
    // ========================================================================

    /// Register a script type in the unit registry.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        self.unit_registry.register_type(entry)?;
        // Rebuild scope to include new type
        self.rebuild_scope();
        Ok(())
    }

    /// Register a script function in the unit registry.
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        self.unit_registry.register_function(entry)?;
        self.rebuild_scope();
        Ok(())
    }

    /// Register a script global in the unit registry.
    pub fn register_global(&mut self, entry: GlobalPropertyEntry) -> Result<(), RegistrationError> {
        self.unit_registry.register_global(entry)?;
        self.rebuild_scope();
        Ok(())
    }

    // ========================================================================
    // Error Handling
    // ========================================================================

    /// Add a compilation error.
    pub fn add_error(&mut self, error: CompilationError) {
        self.errors.push(error);
    }

    /// Check if any errors have been collected.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Take collected errors.
    pub fn take_errors(&mut self) -> Vec<CompilationError> {
        std::mem::take(&mut self.errors)
    }

    /// Get errors as a slice.
    pub fn errors(&self) -> &[CompilationError] {
        &self.errors
    }

    /// Get mutable unit registry for direct manipulation.
    pub fn unit_registry_mut(&mut self) -> &mut SymbolRegistry {
        &mut self.unit_registry
    }

    /// Get unit registry.
    pub fn unit_registry(&self) -> &SymbolRegistry {
        &self.unit_registry
    }

    /// Get global registry.
    pub fn global_registry(&self) -> &SymbolRegistry {
        self.global_registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{ClassEntry, TypeKind};

    #[test]
    fn scope_new_is_empty() {
        let scope = Scope::new();
        assert!(scope.types.is_empty());
        assert!(scope.functions.is_empty());
        assert!(scope.globals.is_empty());
    }

    #[test]
    fn context_resolves_primitives() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let ctx = CompilationContext::new(&registry);

        // Should resolve primitives from global namespace
        assert!(ctx.resolve_type("int").is_some());
        assert!(ctx.resolve_type("float").is_some());
        assert!(ctx.resolve_type("bool").is_some());
    }

    #[test]
    fn context_resolves_qualified_type() {
        let mut registry = SymbolRegistry::new();
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

        // Qualified name should work
        assert!(ctx.resolve_type("Game::Player").is_some());

        // Unqualified shouldn't work from global namespace
        assert!(ctx.resolve_type("Player").is_none());
    }

    #[test]
    fn context_namespace_brings_type_into_scope() {
        let mut registry = SymbolRegistry::new();
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

        // Enter the Game namespace
        ctx.enter_namespace("Game");

        // Now Player should be resolvable
        assert!(ctx.resolve_type("Player").is_some());
    }

    #[test]
    fn context_import_brings_type_into_scope() {
        let mut registry = SymbolRegistry::new();
        let class = ClassEntry::new(
            "Utils",
            vec!["Game".to_string()],
            "Game::Utils",
            TypeHash::from_name("Game::Utils"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import the Game namespace
        ctx.add_import("Game");

        // Now Utils should be resolvable
        assert!(ctx.resolve_type("Utils").is_some());
    }

    #[test]
    fn context_current_namespace_shadows_imports() {
        let mut registry = SymbolRegistry::new();

        // Two classes with same simple name in different namespaces
        let game_player = ClassEntry::new(
            "Player",
            vec!["Game".to_string()],
            "Game::Player",
            TypeHash::from_name("Game::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        let utils_player = ClassEntry::new(
            "Player",
            vec!["Utils".to_string()],
            "Utils::Player",
            TypeHash::from_name("Utils::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(game_player.into()).unwrap();
        registry.register_type(utils_player.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import Utils, then enter Game
        ctx.add_import("Utils");
        ctx.enter_namespace("Game");

        // Player should resolve to Game::Player (current namespace shadows import)
        let resolved = ctx.resolve_type("Player");
        assert_eq!(resolved, Some(TypeHash::from_name("Game::Player")));
    }

    #[test]
    fn context_resolves_functions() {
        use angelscript_core::{
            DataType, FunctionDef, FunctionEntry, FunctionTraits, Visibility, primitives,
        };

        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let def = FunctionDef::new(
            TypeHash::from_function("print", &[primitives::INT32]),
            "print".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry.register_function(FunctionEntry::ffi(def)).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Should resolve function from global namespace
        let resolved = ctx.resolve_function("print");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().len(), 1);
    }

    #[test]
    fn context_resolves_functions_with_namespace() {
        use angelscript_core::{
            DataType, FunctionDef, FunctionEntry, FunctionTraits, Visibility, primitives,
        };

        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let mut def = FunctionDef::new(
            TypeHash::from_function("Game::log", &[primitives::INT32]),
            "log".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        def.namespace = vec!["Game".to_string()];
        registry.register_function(FunctionEntry::ffi(def)).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Not visible from global namespace
        assert!(ctx.resolve_function("log").is_none());

        // Enter Game namespace
        ctx.enter_namespace("Game");

        // Now visible
        let resolved = ctx.resolve_function("log");
        assert!(resolved.is_some());
    }

    #[test]
    fn context_resolves_globals() {
        use angelscript_core::{ConstantValue, GlobalPropertyEntry};

        let mut registry = SymbolRegistry::new();

        let entry = GlobalPropertyEntry::constant("GRAVITY", ConstantValue::Double(9.81));
        registry.register_global(entry).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Should resolve global from global namespace
        assert!(ctx.resolve_global("GRAVITY").is_some());
    }

    #[test]
    fn context_resolves_globals_with_namespace() {
        use angelscript_core::{ConstantValue, GlobalPropertyEntry};

        let mut registry = SymbolRegistry::new();

        let entry = GlobalPropertyEntry::constant("MAX_SPEED", ConstantValue::Double(100.0))
            .with_namespace(vec!["Config".to_string()]);
        registry.register_global(entry).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Not visible from global namespace
        assert!(ctx.resolve_global("MAX_SPEED").is_none());

        // Enter Config namespace
        ctx.enter_namespace("Config");

        // Now visible
        assert!(ctx.resolve_global("MAX_SPEED").is_some());
    }

    #[test]
    fn context_nested_namespace() {
        let mut registry = SymbolRegistry::new();

        let class = ClassEntry::new(
            "Entity",
            vec!["Game".to_string(), "Entities".to_string()],
            "Game::Entities::Entity",
            TypeHash::from_name("Game::Entities::Entity"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Not visible from global
        assert!(ctx.resolve_type("Entity").is_none());

        // Not visible from Game (it's in Game::Entities)
        ctx.enter_namespace("Game");
        assert!(ctx.resolve_type("Entity").is_none());

        // Leave Game, enter Game::Entities
        ctx.exit_namespace();
        ctx.enter_namespace("Game");
        ctx.enter_namespace("Entities");

        // Now visible
        assert!(ctx.resolve_type("Entity").is_some());
    }

    #[test]
    fn context_unit_registry_types() {
        let global_registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&global_registry);

        // Register a type in the unit registry
        let class = ClassEntry::new(
            "LocalClass",
            vec![],
            "LocalClass",
            TypeHash::from_name("LocalClass"),
            TypeKind::ScriptObject,
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        ctx.register_type(class.into()).unwrap();

        // Should be resolvable
        assert!(ctx.resolve_type("LocalClass").is_some());

        // Should be in unit registry, not global
        assert!(ctx.unit_registry().get_by_name("LocalClass").is_some());
        assert!(ctx.global_registry().get_by_name("LocalClass").is_none());
    }

    #[test]
    fn context_leave_namespace() {
        let mut registry = SymbolRegistry::new();

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

        // Enter Game namespace
        ctx.enter_namespace("Game");
        assert!(ctx.resolve_type("Player").is_some());

        // Exit Game namespace
        ctx.exit_namespace();
        assert!(ctx.resolve_type("Player").is_none());
    }

    #[test]
    fn context_current_namespace_path() {
        let registry = SymbolRegistry::new();
        let mut ctx = CompilationContext::new(&registry);

        assert_eq!(ctx.current_namespace(), "");

        ctx.enter_namespace("Game");
        assert_eq!(ctx.current_namespace(), "Game");

        ctx.enter_namespace("Entities");
        assert_eq!(ctx.current_namespace(), "Game::Entities");

        ctx.exit_namespace();
        assert_eq!(ctx.current_namespace(), "Game");

        ctx.exit_namespace();
        assert_eq!(ctx.current_namespace(), "");
    }

    // =========================================================================
    // Ambiguity Detection Tests
    // =========================================================================

    #[test]
    fn context_two_imports_same_type_causes_ambiguity_error() {
        let mut registry = SymbolRegistry::new();

        // Register same-named types in two different namespaces
        let ns_a_player = ClassEntry::new(
            "Player",
            vec!["NamespaceA".to_string()],
            "NamespaceA::Player",
            TypeHash::from_name("NamespaceA::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        let ns_b_player = ClassEntry::new(
            "Player",
            vec!["NamespaceB".to_string()],
            "NamespaceB::Player",
            TypeHash::from_name("NamespaceB::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(ns_a_player.into()).unwrap();
        registry.register_type(ns_b_player.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import both namespaces - should cause ambiguity
        ctx.add_import("NamespaceA");
        assert!(!ctx.has_errors(), "First import should not cause error");

        ctx.add_import("NamespaceB");
        assert!(
            ctx.has_errors(),
            "Second import with conflicting name should cause ambiguity error"
        );

        // Verify it's the right error type
        let errors = ctx.errors();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            CompilationError::AmbiguousSymbol {
                kind,
                name,
                candidates,
                ..
            } => {
                assert_eq!(kind, "type");
                assert_eq!(name, "Player");
                assert!(candidates.contains("NamespaceA::Player"));
                assert!(candidates.contains("NamespaceB::Player"));
            }
            other => panic!("Expected AmbiguousSymbol error, got: {:?}", other),
        }

        // Resolution should still work (returns the last one added due to shadowing)
        // but the error is recorded
        assert!(ctx.resolve_type("Player").is_some());
    }

    #[test]
    fn context_current_namespace_shadows_import_no_error() {
        let mut registry = SymbolRegistry::new();

        // Two classes with same simple name in different namespaces
        let game_player = ClassEntry::new(
            "Player",
            vec!["Game".to_string()],
            "Game::Player",
            TypeHash::from_name("Game::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        let utils_player = ClassEntry::new(
            "Player",
            vec!["Utils".to_string()],
            "Utils::Player",
            TypeHash::from_name("Utils::Player"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(game_player.into()).unwrap();
        registry.register_type(utils_player.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import Utils, then enter Game - current namespace should shadow without error
        ctx.add_import("Utils");
        ctx.enter_namespace("Game");

        // NO error - current namespace legitimately shadows import
        assert!(
            !ctx.has_errors(),
            "Current namespace shadowing import should NOT cause error"
        );

        // Player should resolve to Game::Player
        let resolved = ctx.resolve_type("Player");
        assert_eq!(resolved, Some(TypeHash::from_name("Game::Player")));
    }

    #[test]
    fn context_import_shadows_global_namespace_no_error() {
        let mut registry = SymbolRegistry::new();

        // Type in global namespace
        let global_helper = ClassEntry::new(
            "Helper",
            vec![],
            "Helper",
            TypeHash::from_name("Helper"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        // Type with same name in Utils namespace
        let utils_helper = ClassEntry::new(
            "Helper",
            vec!["Utils".to_string()],
            "Utils::Helper",
            TypeHash::from_name("Utils::Helper"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(global_helper.into()).unwrap();
        registry.register_type(utils_helper.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import Utils - should shadow global namespace without error
        ctx.add_import("Utils");

        // NO error - imports shadow global namespace
        assert!(
            !ctx.has_errors(),
            "Import shadowing global namespace should NOT cause error"
        );

        // Helper should resolve to Utils::Helper (import shadows global)
        let resolved = ctx.resolve_type("Helper");
        assert_eq!(resolved, Some(TypeHash::from_name("Utils::Helper")));
    }

    #[test]
    fn context_two_imports_same_global_causes_ambiguity_error() {
        use angelscript_core::{ConstantValue, GlobalPropertyEntry};

        let mut registry = SymbolRegistry::new();

        // Register same-named globals in two different namespaces
        let config_a = GlobalPropertyEntry::constant("MAX_VALUE", ConstantValue::Int32(100))
            .with_namespace(vec!["ConfigA".to_string()]);
        let config_b = GlobalPropertyEntry::constant("MAX_VALUE", ConstantValue::Int32(200))
            .with_namespace(vec!["ConfigB".to_string()]);
        registry.register_global(config_a).unwrap();
        registry.register_global(config_b).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import both namespaces - should cause ambiguity
        ctx.add_import("ConfigA");
        assert!(!ctx.has_errors(), "First import should not cause error");

        ctx.add_import("ConfigB");
        assert!(
            ctx.has_errors(),
            "Second import with conflicting global should cause ambiguity error"
        );

        // Verify it's the right error type
        let errors = ctx.errors();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            CompilationError::AmbiguousSymbol {
                kind,
                name,
                candidates,
                ..
            } => {
                assert_eq!(kind, "global variable");
                assert_eq!(name, "MAX_VALUE");
                assert!(candidates.contains("ConfigA::MAX_VALUE"));
                assert!(candidates.contains("ConfigB::MAX_VALUE"));
            }
            other => panic!("Expected AmbiguousSymbol error, got: {:?}", other),
        }
    }

    #[test]
    fn context_current_namespace_shadows_imported_global_no_error() {
        use angelscript_core::{ConstantValue, GlobalPropertyEntry};

        let mut registry = SymbolRegistry::new();

        // Global in Utils namespace
        let utils_config = GlobalPropertyEntry::constant("SPEED", ConstantValue::Double(50.0))
            .with_namespace(vec!["Utils".to_string()]);
        // Global with same name in Game namespace
        let game_config = GlobalPropertyEntry::constant("SPEED", ConstantValue::Double(100.0))
            .with_namespace(vec!["Game".to_string()]);
        registry.register_global(utils_config).unwrap();
        registry.register_global(game_config).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import Utils, then enter Game
        ctx.add_import("Utils");
        ctx.enter_namespace("Game");

        // NO error - current namespace shadows import
        assert!(
            !ctx.has_errors(),
            "Current namespace shadowing imported global should NOT cause error"
        );

        // SPEED should resolve to Game::SPEED
        let resolved = ctx.resolve_global("SPEED");
        assert_eq!(resolved, Some(TypeHash::from_name("Game::SPEED")));
    }

    #[test]
    fn context_duplicate_import_is_idempotent() {
        let mut registry = SymbolRegistry::new();

        let class = ClassEntry::new(
            "Widget",
            vec!["UI".to_string()],
            "UI::Widget",
            TypeHash::from_name("UI::Widget"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import same namespace multiple times
        ctx.add_import("UI");
        ctx.add_import("UI");
        ctx.add_import("UI");

        // No errors - duplicate imports are ignored
        assert!(!ctx.has_errors());

        // Widget should resolve correctly
        assert!(ctx.resolve_type("Widget").is_some());
    }

    #[test]
    fn context_ambiguity_error_has_correct_span() {
        let mut registry = SymbolRegistry::new();

        let ns_a = ClassEntry::new(
            "Conflict",
            vec!["A".to_string()],
            "A::Conflict",
            TypeHash::from_name("A::Conflict"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        let ns_b = ClassEntry::new(
            "Conflict",
            vec!["B".to_string()],
            "B::Conflict",
            TypeHash::from_name("B::Conflict"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(ns_a.into()).unwrap();
        registry.register_type(ns_b.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.add_import("A");
        ctx.add_import("B");

        // Error should have a span (even if default for now)
        let errors = ctx.errors();
        assert_eq!(errors.len(), 1);
        let span = errors[0].span();
        // Currently uses Span::default() - but we verify it's accessible
        assert_eq!(span, Span::default());
    }

    #[test]
    fn context_import_shadows_global_namespace_global_no_error() {
        use angelscript_core::{ConstantValue, GlobalPropertyEntry};

        let mut registry = SymbolRegistry::new();

        // Global in global namespace
        let global_config = GlobalPropertyEntry::constant("CONFIG", ConstantValue::Int32(1));
        // Global with same name in Utils namespace
        let utils_config = GlobalPropertyEntry::constant("CONFIG", ConstantValue::Int32(2))
            .with_namespace(vec!["Utils".to_string()]);
        registry.register_global(global_config).unwrap();
        registry.register_global(utils_config).unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import Utils - should shadow global namespace without error
        ctx.add_import("Utils");

        // NO error - imports shadow global namespace
        assert!(
            !ctx.has_errors(),
            "Import shadowing global namespace for globals should NOT cause error"
        );

        // CONFIG should resolve to Utils::CONFIG (import shadows global)
        let resolved = ctx.resolve_global("CONFIG");
        assert_eq!(resolved, Some(TypeHash::from_name("Utils::CONFIG")));
    }

    #[test]
    fn context_take_errors_clears_errors() {
        let mut registry = SymbolRegistry::new();

        let ns_a = ClassEntry::new(
            "Dup",
            vec!["X".to_string()],
            "X::Dup",
            TypeHash::from_name("X::Dup"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        let ns_b = ClassEntry::new(
            "Dup",
            vec!["Y".to_string()],
            "Y::Dup",
            TypeHash::from_name("Y::Dup"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(ns_a.into()).unwrap();
        registry.register_type(ns_b.into()).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.add_import("X");
        ctx.add_import("Y");

        assert!(ctx.has_errors());

        let taken = ctx.take_errors();
        assert_eq!(taken.len(), 1);

        // Errors should be cleared
        assert!(!ctx.has_errors());
        assert!(ctx.errors().is_empty());
    }
}
