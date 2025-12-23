//! Compilation context with namespace-aware symbol resolution.
//!
//! This module provides [`CompilationContext`], which wraps registries and provides
//! symbol resolution using a try-combinations approach.
//!
//! ## Design: Try-Combinations Resolution
//!
//! Instead of maintaining a pre-computed scope that must be rebuilt on namespace
//! changes, we resolve names by trying qualified name combinations at lookup time:
//!
//! 1. If already qualified (contains `::`), try direct lookup
//! 2. Try current namespace prefix (innermost to outermost)
//! 3. Try each import as namespace prefix
//! 4. Try global namespace (unqualified)
//!
//! **Complexity:**
//! - `resolve_type()`: O(d + i) where d = namespace depth, i = import count
//! - `enter_namespace()`: O(1) - just pushes to stack
//! - No scope rebuilding overhead, simpler code, forward references work naturally.

use angelscript_core::{
    CompilationError, DataType, FunctionEntry, GlobalPropertyEntry, RegistrationError, Span,
    TemplateInstanceInfo, TemplateValidation, TypeEntry, TypeHash,
};
use angelscript_registry::SymbolRegistry;

use crate::scope::{LocalScope, LocalVar, VarLookup};
use crate::template::{TemplateCallback, TemplateInstanceCache, instantiate_template_type};

// ============================================================================
// NoOpCallbacks (default template validation)
// ============================================================================

/// Default template callback implementation that accepts all instantiations.
struct NoOpCallbacks;

impl TemplateCallback for NoOpCallbacks {
    fn has_template_callback(&self, _template_hash: TypeHash) -> bool {
        false
    }

    fn validate_template_instance(
        &self,
        _template_hash: TypeHash,
        _info: &TemplateInstanceInfo,
    ) -> TemplateValidation {
        TemplateValidation::valid()
    }
}

// ============================================================================
// CompilationContext
// ============================================================================

/// Compilation context with layered registries and namespace-aware resolution.
///
/// Uses try-combinations approach for symbol resolution: tries qualified name
/// combinations based on current namespace and imports at lookup time.
pub struct CompilationContext<'a> {
    /// Global registry (FFI types, shared types)
    global_registry: &'a SymbolRegistry,

    /// Unit-local registry (script types being compiled)
    unit_registry: SymbolRegistry,

    /// Namespace stack for current position (e.g., ["Game", "Entities"])
    namespace_stack: Vec<String>,

    /// Active using namespace imports
    imports: Vec<String>,

    /// Errors collected during compilation
    errors: Vec<CompilationError>,

    /// Template instance cache for avoiding duplicate instantiation
    template_cache: TemplateInstanceCache,

    /// Stack of local scopes for nested function compilation (e.g., lambdas).
    /// Empty when not in a function. Top of stack is the current function's scope.
    local_scope_stack: Vec<LocalScope>,

    /// String type hash from the string factory (None if not configured)
    string_type_hash: Option<TypeHash>,
}

impl<'a> CompilationContext<'a> {
    /// Create a new compilation context with a reference to the global registry.
    pub fn new(global_registry: &'a SymbolRegistry) -> Self {
        Self {
            global_registry,
            unit_registry: SymbolRegistry::new(),
            namespace_stack: Vec::new(),
            imports: Vec::new(),
            errors: Vec::new(),
            template_cache: TemplateInstanceCache::new(),
            local_scope_stack: Vec::new(),
            string_type_hash: None,
        }
    }

    // ========================================================================
    // String Factory Support
    // ========================================================================

    /// Set the string type hash from the string factory.
    ///
    /// This should be called when setting up the compilation context with
    /// a string factory. String literals will use this type.
    pub fn set_string_type(&mut self, hash: TypeHash) {
        self.string_type_hash = Some(hash);
    }

    /// Get the string type hash (if configured).
    ///
    /// Returns `None` if no string factory has been configured.
    /// String literal compilation will fail if this is `None`.
    pub fn string_type_hash(&self) -> Option<TypeHash> {
        self.string_type_hash
    }

    // ========================================================================
    // Namespace Management
    // ========================================================================

    /// Enter a namespace block: `namespace Game::Entities { ... }`
    pub fn enter_namespace(&mut self, ns: &str) {
        self.namespace_stack.push(ns.to_string());
    }

    /// Exit a namespace block.
    pub fn exit_namespace(&mut self) {
        self.namespace_stack.pop();
    }

    /// Process: `using namespace Game::Utils;`
    pub fn add_import(&mut self, ns: &str) {
        if !self.imports.contains(&ns.to_string()) {
            self.imports.push(ns.to_string());
        }
    }

    /// Get current namespace as qualified string.
    pub fn current_namespace(&self) -> String {
        self.namespace_stack.join("::")
    }

    /// Get active imports.
    pub fn imports(&self) -> &[String] {
        &self.imports
    }

    /// Get current namespace as a vector of segments.
    pub fn namespace_stack(&self) -> &[String] {
        &self.namespace_stack
    }

    // ========================================================================
    // Resolution Methods (try-combinations approach)
    // ========================================================================

    /// Check if a type exists in either registry.
    fn type_exists(&self, hash: TypeHash) -> bool {
        self.unit_registry.get(hash).is_some() || self.global_registry.get(hash).is_some()
    }

    /// Check if a global exists in either registry.
    fn global_exists(&self, hash: TypeHash) -> bool {
        self.unit_registry.get_global(hash).is_some()
            || self.global_registry.get_global(hash).is_some()
    }

    /// Get a type alias by qualified name from either registry.
    fn get_type_alias(&self, qualified_name: &str) -> Option<TypeHash> {
        self.unit_registry
            .get_type_alias(qualified_name)
            .or_else(|| self.global_registry.get_type_alias(qualified_name))
    }

    /// Resolve a type name to its hash using try-combinations.
    ///
    /// Resolution order:
    /// 1. If already qualified (contains `::`), try direct lookup
    /// 2. Try current namespace prefix (innermost to outermost) - if found, return (shadows imports)
    /// 3. Try each import - collect all matches, error if ambiguous
    ///
    /// Complexity: O(d + i) where d = namespace depth, i = import count
    pub fn resolve_type(&self, name: &str) -> Option<TypeHash> {
        self.resolve_type_checked(name).ok().flatten()
    }

    /// Resolve a type name with ambiguity checking.
    ///
    /// Resolution order:
    /// 1. If already qualified (contains `::`), try direct lookup (types, then aliases)
    /// 2. Try current namespace hierarchy (innermost to outermost, NOT global)
    /// 3. Try imports - collect all matches, error if ambiguous
    /// 4. Fall back to global namespace
    /// 5. Check type aliases with the same resolution order
    ///
    /// Returns `Err` with an ambiguity error if the name matches in multiple imports.
    /// Returns `Ok(None)` if not found, `Ok(Some(hash))` if found unambiguously.
    pub fn resolve_type_checked(&self, name: &str) -> Result<Option<TypeHash>, CompilationError> {
        // 1. If already qualified, try direct lookup
        if name.contains("::") {
            let hash = TypeHash::from_name(name);
            if self.type_exists(hash) {
                return Ok(Some(hash));
            }
            // Also check qualified type aliases
            if let Some(target_hash) = self.get_type_alias(name) {
                return Ok(Some(target_hash));
            }
            return Ok(None);
        }

        // 2. Try current namespace hierarchy (innermost to outermost, NOT global)
        // If found, shadows imports and global
        for i in (1..=self.namespace_stack.len()).rev() {
            let prefix = self.namespace_stack[..i].join("::");
            let qualified = format!("{}::{}", prefix, name);
            let hash = TypeHash::from_name(&qualified);
            if self.type_exists(hash) {
                return Ok(Some(hash));
            }
            // Check type aliases in this namespace
            if let Some(target_hash) = self.get_type_alias(&qualified) {
                return Ok(Some(target_hash));
            }
        }

        // 3. Try imports - collect all matches to detect ambiguity
        let mut found: Option<(TypeHash, String)> = None;
        for import in &self.imports {
            let qualified = format!("{}::{}", import, name);
            let hash = TypeHash::from_name(&qualified);
            if self.type_exists(hash) {
                if let Some((existing_hash, ref existing_qualified)) = found {
                    if existing_hash != hash {
                        // Ambiguity: same simple name in different imports
                        return Err(CompilationError::AmbiguousSymbol {
                            kind: "type".to_string(),
                            name: name.to_string(),
                            candidates: format!("{}, {}", existing_qualified, qualified),
                            span: Span::default(),
                        });
                    }
                } else {
                    found = Some((hash, qualified.clone()));
                }
            }
            // Also check type aliases in imports
            if let Some(target_hash) = self.get_type_alias(&qualified) {
                if let Some((existing_hash, ref existing_qualified)) = found {
                    if existing_hash != target_hash {
                        return Err(CompilationError::AmbiguousSymbol {
                            kind: "type".to_string(),
                            name: name.to_string(),
                            candidates: format!("{}, {}", existing_qualified, qualified),
                            span: Span::default(),
                        });
                    }
                } else {
                    found = Some((target_hash, qualified));
                }
            }
        }

        if let Some((hash, _)) = found {
            return Ok(Some(hash));
        }

        // 4. Fall back to global namespace
        let hash = TypeHash::from_name(name);
        if self.type_exists(hash) {
            return Ok(Some(hash));
        }

        // 5. Check type aliases in global namespace
        if let Some(target_hash) = self.get_type_alias(name) {
            return Ok(Some(target_hash));
        }

        Ok(None)
    }

    /// Resolve a function name to all matching overloads using try-combinations.
    ///
    /// Unlike types, functions can have overloads, so we collect all matches.
    /// If found in current namespace, imports are not searched (shadowing).
    ///
    /// Complexity: O(d + i) where d = namespace depth, i = import count
    pub fn resolve_function(&self, name: &str) -> Option<Vec<TypeHash>> {
        // If already qualified, try direct lookup
        if name.contains("::") {
            if let Some(idx) = name.rfind("::") {
                let ns = &name[..idx];
                let simple = &name[idx + 2..];
                let mut results = Vec::new();

                // Check unit registry
                if let Some(funcs) = self.unit_registry.get_namespace_functions(ns)
                    && let Some(qname) = funcs.get(simple)
                    && let Some(entries) = self.unit_registry.get_functions(qname)
                {
                    results.extend(entries.iter().map(|e| e.def.func_hash));
                }

                // Check global registry
                if let Some(funcs) = self.global_registry.get_namespace_functions(ns)
                    && let Some(qname) = funcs.get(simple)
                    && let Some(entries) = self.global_registry.get_functions(qname)
                {
                    for entry in entries {
                        if !results.contains(&entry.def.func_hash) {
                            results.push(entry.def.func_hash);
                        }
                    }
                }

                if results.is_empty() {
                    return None;
                }
                return Some(results);
            }
            return None;
        }

        let mut results = Vec::new();

        // Helper to add functions from a namespace
        let add_from_namespace = |ns: &str, results: &mut Vec<TypeHash>| -> bool {
            let mut found = false;
            // Check unit registry
            if let Some(funcs) = self.unit_registry.get_namespace_functions(ns)
                && let Some(qname) = funcs.get(name)
                && let Some(entries) = self.unit_registry.get_functions(qname)
            {
                for entry in entries {
                    if !results.contains(&entry.def.func_hash) {
                        results.push(entry.def.func_hash);
                        found = true;
                    }
                }
            }
            // Check global registry
            if let Some(funcs) = self.global_registry.get_namespace_functions(ns)
                && let Some(qname) = funcs.get(name)
                && let Some(entries) = self.global_registry.get_functions(qname)
            {
                for entry in entries {
                    if !results.contains(&entry.def.func_hash) {
                        results.push(entry.def.func_hash);
                        found = true;
                    }
                }
            }
            found
        };

        // Try current namespace hierarchy (innermost to outermost, NOT global)
        let mut found_in_current_ns = false;
        for i in (1..=self.namespace_stack.len()).rev() {
            let prefix = self.namespace_stack[..i].join("::");
            if add_from_namespace(&prefix, &mut results) {
                found_in_current_ns = true;
            }
        }

        // If found in current namespace, don't check imports or global (shadowing)
        if !found_in_current_ns {
            // Try imports
            let mut found_in_imports = false;
            for import in &self.imports {
                if add_from_namespace(import, &mut results) {
                    found_in_imports = true;
                }
            }

            // Fall back to global namespace
            if !found_in_imports {
                add_from_namespace("", &mut results);
            }
        }

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Resolve a global variable name to its hash using try-combinations.
    ///
    /// Complexity: O(d + i) where d = namespace depth, i = import count
    pub fn resolve_global(&self, name: &str) -> Option<TypeHash> {
        self.resolve_global_checked(name).ok().flatten()
    }

    /// Resolve a global variable name with ambiguity checking.
    ///
    /// Resolution order:
    /// 1. If already qualified (contains `::`), try direct lookup
    /// 2. Try current namespace hierarchy (innermost to outermost, NOT global)
    /// 3. Try imports - collect all matches, error if ambiguous
    /// 4. Fall back to global namespace
    ///
    /// Returns `Err` with an ambiguity error if the name matches in multiple imports.
    pub fn resolve_global_checked(&self, name: &str) -> Result<Option<TypeHash>, CompilationError> {
        // 1. If already qualified, try direct lookup
        if name.contains("::") {
            let hash = TypeHash::from_name(name);
            if self.global_exists(hash) {
                return Ok(Some(hash));
            }
            return Ok(None);
        }

        // 2. Try current namespace hierarchy (innermost to outermost, NOT global)
        // If found, shadows imports and global
        for i in (1..=self.namespace_stack.len()).rev() {
            let prefix = self.namespace_stack[..i].join("::");
            let qualified = format!("{}::{}", prefix, name);
            let hash = TypeHash::from_name(&qualified);
            if self.global_exists(hash) {
                return Ok(Some(hash));
            }
        }

        // 3. Try imports - detect ambiguity
        let mut found: Option<(TypeHash, String)> = None;
        for import in &self.imports {
            let qualified = format!("{}::{}", import, name);
            let hash = TypeHash::from_name(&qualified);
            if self.global_exists(hash) {
                if let Some((existing_hash, ref existing_qualified)) = found {
                    if existing_hash != hash {
                        return Err(CompilationError::AmbiguousSymbol {
                            kind: "global variable".to_string(),
                            name: name.to_string(),
                            candidates: format!("{}, {}", existing_qualified, qualified),
                            span: Span::default(),
                        });
                    }
                } else {
                    found = Some((hash, qualified));
                }
            }
        }

        if let Some((hash, _)) = found {
            return Ok(Some(hash));
        }

        // 4. Fall back to global namespace
        let hash = TypeHash::from_name(name);
        if self.global_exists(hash) {
            return Ok(Some(hash));
        }

        Ok(None)
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

    /// Validate that a type can be used for variable declarations.
    ///
    /// - Mixin classes cannot be used at all (they're not real types)
    /// - Interfaces can only be used as handles (e.g., `IDrawable@`, not `IDrawable x;`)
    ///
    /// Returns an error if the type cannot be used. This is O(1) since `is_mixin` and
    /// `is_interface` are cached in the DataType during type resolution.
    pub fn validate_instantiable_type(
        &self,
        data_type: &DataType,
        span: Span,
        context: &str,
    ) -> Result<(), CompilationError> {
        // Mixins cannot be used at all - they're not real types
        if data_type.is_mixin {
            let type_name = self
                .get_type(data_type.type_hash)
                .and_then(|e| e.as_class())
                .map(|c| c.name.as_str())
                .unwrap_or("<unknown>");
            return Err(CompilationError::InvalidOperation {
                message: format!(
                    "cannot use mixin class '{}' {}; mixins are not instantiable types",
                    type_name, context
                ),
                span,
            });
        }

        // Interfaces can only be used as handles
        if data_type.is_interface && !data_type.is_handle {
            let type_name = self
                .get_type(data_type.type_hash)
                .and_then(|e| e.as_interface())
                .map(|i| i.name.as_str())
                .unwrap_or("<unknown>");
            return Err(CompilationError::InvalidOperation {
                message: format!(
                    "cannot use interface '{}' {}; interfaces can only be used as handles ({}@)",
                    type_name, context, type_name
                ),
                span,
            });
        }

        Ok(())
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

    /// Find all callable methods on a type by name, including inherited.
    ///
    /// Uses the vtable for lookup for classes (which contains all methods own + inherited),
    /// and the itable for interfaces.
    ///
    /// If `is_const_object` is true, only returns const methods (callable on const objects).
    /// If `is_const_object` is false, returns all methods.
    pub fn find_methods(
        &self,
        type_hash: TypeHash,
        name: &str,
        is_const_object: bool,
    ) -> Vec<TypeHash> {
        let candidates = {
            // Check class in unit registry first
            if let Some(class) = self.unit_registry.get(type_hash).and_then(|e| e.as_class()) {
                class.find_callable_methods(name)
            }
            // Check class in global registry
            else if let Some(class) = self
                .global_registry
                .get(type_hash)
                .and_then(|e| e.as_class())
            {
                class.find_callable_methods(name)
            }
            // Check interface in unit registry
            else if let Some(iface) = self
                .unit_registry
                .get(type_hash)
                .and_then(|e| e.as_interface())
            {
                iface.itable.find_methods(name)
            }
            // Check interface in global registry
            else if let Some(iface) = self
                .global_registry
                .get(type_hash)
                .and_then(|e| e.as_interface())
            {
                iface.itable.find_methods(name)
            } else {
                return Vec::new();
            }
        };

        if !is_const_object {
            return candidates;
        }

        // Filter to only const methods for const objects
        candidates
            .into_iter()
            .filter(|&hash| self.get_function(hash).is_some_and(|f| f.def.is_const()))
            .collect()
    }

    /// Check if `derived` is derived from `base` in the class hierarchy.
    ///
    /// This walks the inheritance chain from `derived` upward, checking if
    /// `base` is found anywhere in the chain. Returns `false` if:
    /// - `derived == base` (same type is not "derived from" itself)
    /// - `derived` is not a class type
    /// - `base` is not in `derived`'s inheritance chain
    pub fn is_type_derived_from(&self, derived: TypeHash, base: TypeHash) -> bool {
        let mut current = derived;
        while let Some(class) = self.get_type(current).and_then(|t| t.as_class()) {
            if let Some(base_class) = class.base_class {
                if base_class == base {
                    return true;
                }
                current = base_class;
            } else {
                break;
            }
        }
        false
    }

    // ========================================================================
    // Registration (for unit registry)
    // ========================================================================

    /// Register a script type in the unit registry.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        self.unit_registry.register_type(entry)
    }

    /// Register a script function in the unit registry.
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        self.unit_registry.register_function(entry)
    }

    /// Register a script global in the unit registry.
    pub fn register_global(&mut self, entry: GlobalPropertyEntry) -> Result<(), RegistrationError> {
        self.unit_registry.register_global(entry)
    }

    // ========================================================================
    // Template Instantiation
    // ========================================================================

    /// Instantiate a template type with concrete type arguments.
    ///
    /// Returns the hash of the instantiated type. Uses caching to avoid
    /// duplicate instantiation and respects FFI specializations.
    pub fn instantiate_template(
        &mut self,
        template_hash: TypeHash,
        type_args: &[DataType],
        span: Span,
    ) -> Result<TypeHash, CompilationError> {
        // Use NoOpCallbacks for now - validation callbacks can be added later
        let callbacks = NoOpCallbacks;
        instantiate_template_type(
            template_hash,
            type_args,
            span,
            &mut self.template_cache,
            &mut self.unit_registry,
            self.global_registry,
            &callbacks,
        )
    }

    // ========================================================================
    // Local Scope Management (for function compilation)
    // ========================================================================

    /// Begin compiling a function - pushes a new local scope onto the stack.
    ///
    /// This supports nested function compilation (e.g., lambdas inside functions).
    pub fn begin_function(&mut self) {
        self.local_scope_stack.push(LocalScope::new());
    }

    /// End function compilation - pops and returns the current local scope.
    ///
    /// Returns `None` if not in a function.
    pub fn end_function(&mut self) -> Option<LocalScope> {
        self.local_scope_stack.pop()
    }

    /// Check if currently compiling a function.
    pub fn in_function(&self) -> bool {
        !self.local_scope_stack.is_empty()
    }

    /// Get the current function scope nesting depth.
    ///
    /// Returns 0 if not in a function, 1 for top-level function, 2+ for nested lambdas.
    pub fn function_depth(&self) -> usize {
        self.local_scope_stack.len()
    }

    /// Enter a nested block scope (if body, loop body, etc.).
    ///
    /// Panics if not in a function.
    pub fn push_local_scope(&mut self) {
        self.local_scope_stack
            .last_mut()
            .expect("push_local_scope called outside function")
            .push_scope();
    }

    /// Exit the current block scope.
    ///
    /// Returns the variables that went out of scope, for cleanup bytecode emission.
    /// Panics if not in a function.
    pub fn pop_local_scope(&mut self) -> Vec<crate::scope::LocalVar> {
        self.local_scope_stack
            .last_mut()
            .expect("pop_local_scope called outside function")
            .pop_scope()
    }

    /// Declare a local variable in the current scope.
    ///
    /// Returns the stack slot, or error if redeclared or type is not instantiable.
    /// Panics if not in a function.
    pub fn declare_local(
        &mut self,
        name: String,
        data_type: DataType,
        is_const: bool,
        span: Span,
    ) -> Result<u32, CompilationError> {
        // Validate type is instantiable (not a mixin)
        self.validate_instantiable_type(&data_type, span, "as local variable type")?;

        self.local_scope_stack
            .last_mut()
            .expect("declare_local called outside function")
            .declare(name, data_type, is_const, span)
    }

    /// Declare a function parameter.
    ///
    /// Parameters are always initialized.
    /// Panics if not in a function.
    pub fn declare_param(
        &mut self,
        name: String,
        data_type: DataType,
        is_const: bool,
        span: Span,
    ) -> Result<u32, CompilationError> {
        // Validate type is instantiable (not a mixin)
        self.validate_instantiable_type(&data_type, span, "as function parameter type")?;

        self.local_scope_stack
            .last_mut()
            .expect("declare_param called outside function")
            .declare_param(name, data_type, is_const, span)
    }

    /// Mark a local variable as initialized.
    ///
    /// Panics if not in a function.
    pub fn mark_local_initialized(&mut self, name: &str) {
        self.local_scope_stack
            .last_mut()
            .expect("mark_local_initialized called outside function")
            .mark_initialized(name);
    }

    /// Look up a local variable by name.
    ///
    /// Returns `None` if not in a function or variable not found.
    pub fn get_local(&self, name: &str) -> Option<&LocalVar> {
        self.local_scope_stack.last()?.get(name)
    }

    /// Look up a variable, capturing from parent scopes if in a lambda.
    ///
    /// Returns `None` if not in a function or variable not found.
    pub fn get_local_or_capture(&mut self, name: &str) -> Option<VarLookup> {
        self.local_scope_stack.last_mut()?.get_or_capture(name)
    }

    /// Get the local scope (if in a function).
    pub fn local_scope(&self) -> Option<&LocalScope> {
        self.local_scope_stack.last()
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
    use angelscript_core::{ClassEntry, RefModifier, TypeKind, primitives};

    #[test]
    fn context_resolves_primitives() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();

        let ctx = CompilationContext::new(&registry);

        // Should resolve primitives from global namespace
        assert_eq!(ctx.resolve_type("int"), Some(primitives::INT32));
        assert_eq!(ctx.resolve_type("float"), Some(primitives::FLOAT));
        assert_eq!(ctx.resolve_type("bool"), Some(primitives::BOOL));
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
        assert_eq!(
            ctx.resolve_type("Game::Player"),
            Some(TypeHash::from_name("Game::Player"))
        );

        // Unqualified shouldn't work from global namespace
        assert!(
            ctx.resolve_type("Player").is_none(),
            "Player should not resolve without namespace"
        );
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
        assert_eq!(
            ctx.resolve_type("Player"),
            Some(TypeHash::from_name("Game::Player"))
        );
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
        assert_eq!(
            ctx.resolve_type("Utils"),
            Some(TypeHash::from_name("Game::Utils"))
        );
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
        let resolved = ctx.resolve_function("print").expect("print should resolve");
        assert_eq!(resolved.len(), 1);
        // Verify the resolved hash matches what we registered
        assert_eq!(
            resolved[0],
            TypeHash::from_function("print", &[primitives::INT32])
        );
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
        assert!(
            ctx.resolve_function("log").is_none(),
            "log should not resolve from global namespace"
        );

        // Enter Game namespace
        ctx.enter_namespace("Game");

        // Now visible
        let resolved = ctx
            .resolve_function("log")
            .expect("log should resolve in Game namespace");
        assert_eq!(resolved.len(), 1);
        // Verify the resolved hash matches what we registered
        assert_eq!(
            resolved[0],
            TypeHash::from_function("Game::log", &[primitives::INT32])
        );
    }

    #[test]
    fn context_resolves_globals() {
        use angelscript_core::{ConstantValue, GlobalPropertyEntry};

        let mut registry = SymbolRegistry::new();

        let entry = GlobalPropertyEntry::constant("GRAVITY", ConstantValue::Double(9.81));
        registry.register_global(entry).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Should resolve global from global namespace
        let global = ctx
            .resolve_global("GRAVITY")
            .expect("GRAVITY should resolve");
        // Verify the resolved hash matches what we registered
        assert_eq!(global, TypeHash::from_name("GRAVITY"));
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
        assert!(
            ctx.resolve_global("MAX_SPEED").is_none(),
            "MAX_SPEED should not resolve from global namespace"
        );

        // Enter Config namespace
        ctx.enter_namespace("Config");

        // Now visible
        let global = ctx
            .resolve_global("MAX_SPEED")
            .expect("MAX_SPEED should resolve in Config namespace");
        // Verify the resolved hash matches what we registered
        assert_eq!(global, TypeHash::from_name("Config::MAX_SPEED"));
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
        assert!(
            ctx.resolve_type("Entity").is_none(),
            "Entity should not resolve from global namespace"
        );

        // Not visible from Game (it's in Game::Entities)
        ctx.enter_namespace("Game");
        assert!(
            ctx.resolve_type("Entity").is_none(),
            "Entity should not resolve from Game namespace"
        );

        // Leave Game, enter Game::Entities
        ctx.exit_namespace();
        ctx.enter_namespace("Game");
        ctx.enter_namespace("Entities");

        // Now visible
        assert_eq!(
            ctx.resolve_type("Entity"),
            Some(TypeHash::from_name("Game::Entities::Entity"))
        );
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

        // With try-combinations approach, types are resolvable immediately after registration
        assert_eq!(
            ctx.resolve_type("LocalClass"),
            Some(TypeHash::from_name("LocalClass"))
        );

        // Should be in unit registry, not global
        assert!(
            ctx.unit_registry().get_by_name("LocalClass").is_some(),
            "LocalClass should be in unit registry"
        );
        assert!(
            ctx.global_registry().get_by_name("LocalClass").is_none(),
            "LocalClass should not be in global registry"
        );
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
        assert_eq!(
            ctx.resolve_type("Player"),
            Some(TypeHash::from_name("Game::Player"))
        );

        // Exit Game namespace
        ctx.exit_namespace();
        assert!(
            ctx.resolve_type("Player").is_none(),
            "Player should not resolve after exiting namespace"
        );
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

        // Import both namespaces
        ctx.add_import("NamespaceA");
        ctx.add_import("NamespaceB");

        // Ambiguity is detected at resolution time (try-combinations approach)
        let result = ctx.resolve_type_checked("Player");
        assert!(
            result.is_err(),
            "Resolution should return ambiguity error when same name in multiple imports"
        );

        // Verify it's the right error type
        match result.unwrap_err() {
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

        // resolve_type (non-checked) returns None on ambiguity
        assert!(
            ctx.resolve_type("Player").is_none(),
            "resolve_type should return None on ambiguity"
        );
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

        // Import both namespaces
        ctx.add_import("ConfigA");
        ctx.add_import("ConfigB");

        // Ambiguity is detected at resolution time (try-combinations approach)
        let result = ctx.resolve_global_checked("MAX_VALUE");
        assert!(
            result.is_err(),
            "Resolution should return ambiguity error when same name in multiple imports"
        );

        // Verify it's the right error type
        match result.unwrap_err() {
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

        // resolve_global (non-checked) returns None on ambiguity
        assert!(
            ctx.resolve_global("MAX_VALUE").is_none(),
            "resolve_global should return None on ambiguity"
        );
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
        assert_eq!(
            ctx.resolve_type("Widget"),
            Some(TypeHash::from_name("UI::Widget"))
        );
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

        // Ambiguity is detected at resolution time
        let result = ctx.resolve_type_checked("Conflict");
        assert!(result.is_err());

        // Error should have a span (even if default for now)
        let err = result.unwrap_err();
        let span = err.span();
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
        let registry = SymbolRegistry::new();
        let mut ctx = CompilationContext::new(&registry);

        // Add an error directly
        ctx.add_error(CompilationError::AmbiguousSymbol {
            kind: "type".to_string(),
            name: "Dup".to_string(),
            candidates: "X::Dup, Y::Dup".to_string(),
            span: Span::default(),
        });

        assert!(ctx.has_errors());

        let taken = ctx.take_errors();
        assert_eq!(taken.len(), 1);

        // Errors should be cleared
        assert!(!ctx.has_errors());
        assert!(ctx.errors().is_empty());
    }

    // =========================================================================
    // CompilationContext + LocalScope Integration Tests
    // =========================================================================

    #[test]
    fn context_local_scope_lifecycle() {
        use angelscript_core::primitives;

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        // Not in function initially
        assert!(!ctx.in_function());
        assert!(
            ctx.local_scope().is_none(),
            "local_scope should be None before begin_function"
        );

        // Begin function
        ctx.begin_function();
        assert!(ctx.in_function());
        assert!(
            ctx.local_scope().is_some(),
            "local_scope should be Some after begin_function"
        );

        // Declare a variable
        let slot = ctx
            .declare_local(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();
        assert_eq!(slot, 0);

        // Look it up
        let var = ctx.get_local("x").expect("x should be declared");
        assert_eq!(var.slot, 0);
        assert_eq!(var.data_type.type_hash, primitives::INT32);

        // End function
        let scope = ctx.end_function().unwrap();
        assert_eq!(scope.frame_size(), 1);
        assert!(!ctx.in_function());
    }

    #[test]
    fn context_nested_local_scopes() {
        use angelscript_core::primitives;

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        ctx.begin_function();

        // Declare in function scope
        ctx.declare_local(
            "outer".into(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        )
        .unwrap();

        // Enter block
        ctx.push_local_scope();
        ctx.declare_local(
            "inner".into(),
            DataType::simple(primitives::FLOAT),
            false,
            Span::default(),
        )
        .unwrap();

        // Both visible
        let outer = ctx.get_local("outer").expect("outer should be visible");
        assert_eq!(outer.slot, 0);
        let inner = ctx.get_local("inner").expect("inner should be visible");
        assert_eq!(inner.slot, 1);

        // Exit block
        ctx.pop_local_scope();

        // Only outer visible
        assert!(
            ctx.get_local("outer").is_some(),
            "outer should still be visible after popping scope"
        );
        assert!(
            ctx.get_local("inner").is_none(),
            "inner should not be visible after popping scope"
        );

        ctx.end_function();
    }

    #[test]
    fn context_nested_function_scopes() {
        use angelscript_core::primitives;

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);

        // Outer function
        ctx.begin_function();
        assert_eq!(ctx.function_depth(), 1);

        ctx.declare_local(
            "outer_var".into(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        )
        .unwrap();

        // Nested lambda (inner function)
        ctx.begin_function();
        assert_eq!(ctx.function_depth(), 2);

        // Lambda has its own isolated scope - outer_var not visible
        assert!(ctx.get_local("outer_var").is_none());

        ctx.declare_local(
            "inner_var".into(),
            DataType::simple(primitives::BOOL),
            false,
            Span::default(),
        )
        .unwrap();
        assert!(ctx.get_local("inner_var").is_some());

        // End lambda
        ctx.end_function();
        assert_eq!(ctx.function_depth(), 1);

        // Back in outer function - outer_var visible again, inner_var not
        assert!(ctx.get_local("outer_var").is_some());
        assert!(ctx.get_local("inner_var").is_none());

        ctx.end_function();
        assert_eq!(ctx.function_depth(), 0);
    }

    #[test]
    fn declare_local_rejects_mixin_type() {
        let registry = SymbolRegistry::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Create a DataType with is_mixin = true
        let mixin_type = DataType {
            type_hash: TypeHash::from_name("TestMixin"),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: true,
            is_interface: false,
            is_enum: false,
        };

        let result = ctx.declare_local("x".into(), mixin_type, false, Span::default());

        assert!(result.is_err());
        if let Err(CompilationError::InvalidOperation { message, .. }) = result {
            assert!(message.contains("mixin"));
            assert!(message.contains("instantiable"));
        } else {
            panic!("Expected InvalidOperation error");
        }

        ctx.end_function();
    }

    #[test]
    fn declare_param_rejects_mixin_type() {
        let registry = SymbolRegistry::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Create a DataType with is_mixin = true
        let mixin_type = DataType {
            type_hash: TypeHash::from_name("TestMixin"),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: true,
            is_interface: false,
            is_enum: false,
        };

        let result = ctx.declare_param("param".into(), mixin_type, false, Span::default());

        assert!(result.is_err());
        if let Err(CompilationError::InvalidOperation { message, .. }) = result {
            assert!(message.contains("mixin"));
            assert!(message.contains("parameter"));
        } else {
            panic!("Expected InvalidOperation error");
        }

        ctx.end_function();
    }

    #[test]
    fn declare_local_accepts_non_mixin_type() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Regular int type should be accepted
        let int_type = DataType::simple(primitives::INT32);
        let result = ctx.declare_local("x".into(), int_type, false, Span::default());

        assert!(result.is_ok());

        ctx.end_function();
    }

    // =========================================================================
    // Type Alias Resolution Tests
    // =========================================================================

    #[test]
    fn context_resolves_type_alias_global() {
        let mut registry = SymbolRegistry::with_primitives();
        registry
            .register_type_alias("EntityId", &[], primitives::INT32)
            .unwrap();

        let ctx = CompilationContext::new(&registry);

        // Should resolve typedef to target type
        let resolved = ctx.resolve_type("EntityId");
        assert_eq!(resolved, Some(primitives::INT32));
    }

    #[test]
    fn context_resolves_type_alias_in_namespace() {
        let mut registry = SymbolRegistry::with_primitives();
        registry
            .register_type_alias("EntityId", &["Game".to_string()], primitives::INT32)
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Not visible from global namespace
        assert!(
            ctx.resolve_type("EntityId").is_none(),
            "EntityId should not resolve from global namespace"
        );

        // Enter the Game namespace
        ctx.enter_namespace("Game");

        // Now visible
        let resolved = ctx.resolve_type("EntityId");
        assert_eq!(resolved, Some(primitives::INT32));
    }

    #[test]
    fn context_resolves_qualified_type_alias() {
        let mut registry = SymbolRegistry::with_primitives();
        registry
            .register_type_alias("EntityId", &["Game".to_string()], primitives::INT32)
            .unwrap();

        let ctx = CompilationContext::new(&registry);

        // Should resolve qualified alias from global namespace
        let resolved = ctx.resolve_type("Game::EntityId");
        assert_eq!(resolved, Some(primitives::INT32));
    }

    #[test]
    fn context_type_shadows_type_alias() {
        let mut registry = SymbolRegistry::with_primitives();

        // Register a class named "EntityId"
        let class = ClassEntry::new(
            "EntityId",
            vec![],
            "EntityId",
            TypeHash::from_name("EntityId"),
            TypeKind::reference(),
            angelscript_core::entries::TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        // Also register a typedef with the same name (unusual but possible)
        registry
            .register_type_alias("EntityId", &["Other".to_string()], primitives::INT32)
            .unwrap();

        let ctx = CompilationContext::new(&registry);

        // The type should have priority over the alias in global namespace
        let resolved = ctx.resolve_type("EntityId");
        assert_eq!(resolved, Some(TypeHash::from_name("EntityId")));

        // The namespaced alias should still work
        let resolved = ctx.resolve_type("Other::EntityId");
        assert_eq!(resolved, Some(primitives::INT32));
    }

    #[test]
    fn context_resolves_type_alias_via_import() {
        let mut registry = SymbolRegistry::with_primitives();
        registry
            .register_type_alias("EntityId", &["Game".to_string()], primitives::INT32)
            .unwrap();

        let mut ctx = CompilationContext::new(&registry);

        // Import the Game namespace
        ctx.add_import("Game");

        // Should resolve via import
        let resolved = ctx.resolve_type("EntityId");
        assert_eq!(resolved, Some(primitives::INT32));
    }
}
