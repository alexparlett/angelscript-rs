# Task 33: Compilation Context

## Overview

Create the `CompilationContext` that provides **layered registry lookup** for compilation. Uses a global TypeRegistry (FFI + templates + shared types) and a per-unit TypeRegistry (non-shared script types).

## Goals

1. Layered lookup: unit registry first, then global registry
2. Namespace management (current namespace, imports)
3. Template instantiation via `TemplateInstantiator` (Task 35)
4. Error collection during compilation
5. Clean memory management (drop unit registry when done)

## Dependencies

- Task 31: Compiler Foundation

## Architecture

```
┌────────────────────────────────────┐
│ TypeRegistry (Global)              │  ← global_registry (immutable ref)
│ - FFI types/functions              │
│ - Template instances               │
│ - Shared script types              │
└──────────────────┬─────────────────┘
                   │
                   ▼
           CompilationContext
                   │
                   ▼
┌────────────────────────────────────┐
│ TypeRegistry (Unit)                │  ← unit_registry (owned)
│ - Non-shared script types          │
│ - Local functions                  │
└────────────────────────────────────┘
```

## Files to Create/Modify

```
crates/angelscript-compiler/src/
├── context.rs             # CompilationContext
├── script_defs.rs         # ScriptTypeDef, ScriptFunctionDef
└── lib.rs                 # Add modules
```

## Detailed Implementation

### 1. Script Definitions (script_defs.rs)

During registration pass, we collect script definitions before adding to registry:

```rust
use angelscript_core::{DataType, Span, TypeHash, Visibility};
use angelscript_parser::ast::AstIndex;

/// A script-defined type pending full registration.
#[derive(Debug, Clone)]
pub struct ScriptTypeDef {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub kind: ScriptTypeKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ScriptTypeKind {
    Class {
        base_class: Option<TypeHash>,
        interfaces: Vec<TypeHash>,
        is_final: bool,
        is_abstract: bool,
    },
    Interface {
        methods: Vec<TypeHash>,
    },
    Enum {
        underlying: TypeHash,
        values: Vec<(String, i64)>,
    },
    Funcdef {
        params: Vec<DataType>,
        return_type: DataType,
    },
}

/// A script-defined function pending compilation.
#[derive(Debug, Clone)]
pub struct ScriptFunctionDef {
    pub name: String,
    pub qualified_name: String,
    pub func_hash: TypeHash,
    pub params: Vec<ScriptParam>,
    pub return_type: DataType,
    pub object_type: Option<TypeHash>,  // None for free functions
    pub is_const: bool,
    pub visibility: Visibility,
    pub body_ast: Option<AstIndex>,     // Index into AST for body compilation
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ScriptParam {
    pub name: String,
    pub data_type: DataType,
    pub has_default: bool,
    pub default_ast: Option<AstIndex>,  // Index into AST for default value
}
```

### 2. Compilation Context (context.rs)

```rust
use angelscript_core::{DataType, Span, TypeEntry, TypeHash, UnitId, FunctionEntry, GlobalPropertyEntry};
use angelscript_registry::TypeRegistry;
use rustc_hash::FxHashMap;

use crate::error::{CompileError, Result};
use crate::script_defs::{ScriptFunctionDef, ScriptTypeDef};

/// Compilation context providing **layered registry lookup**.
///
/// Uses two registries:
/// - `global_registry`: FFI types, template instances, shared script types (immutable ref)
/// - `unit_registry`: Non-shared script types for this compilation unit (owned)
///
/// Lookup order: unit_registry first, then global_registry.
///
/// ## Performance: Namespace Caching
///
/// To avoid repeated `format!()` allocations in hot lookup paths, the context
/// caches the current namespace string (`current_namespace_cached`) and imported
/// namespace strings (`import_caches`). These are updated only when namespaces
/// change via `enter_namespace()` / `exit_namespace()` / `add_import()`.
pub struct CompilationContext<'a> {
    /// Global registry (FFI + templates + shared) - immutable reference
    pub global_registry: &'a TypeRegistry,

    /// Per-unit registry (non-shared script types) - owned
    pub unit_registry: TypeRegistry,

    /// Current compilation unit
    pub unit_id: UnitId,

    /// Current namespace path during compilation
    namespace_stack: Vec<String>,

    /// Cached current namespace string (updated on enter/exit_namespace)
    /// Avoids repeated join("::") calls in hot paths.
    current_namespace_cached: String,

    /// Imported namespaces (from 'using' declarations)
    imports: Vec<Vec<String>>,

    /// Cached import strings (updated on add_import/clear_imports)
    /// Each entry is the joined namespace path, e.g., "std::math"
    import_caches: Vec<String>,

    /// Collected errors
    errors: Vec<CompileError>,
}

impl<'a> CompilationContext<'a> {
    /// Create a new compilation context.
    pub fn new(global_registry: &'a TypeRegistry, unit_id: UnitId) -> Self {
        Self {
            global_registry,
            unit_registry: TypeRegistry::new(),
            unit_id,
            namespace_stack: Vec::new(),
            current_namespace_cached: String::new(),
            imports: Vec::new(),
            import_caches: Vec::new(),
            errors: Vec::new(),
        }
    }

    // ==========================================================================
    // Type Lookup (Layered: unit first, then global)
    // ==========================================================================

    /// Get a type entry by hash. Checks unit registry first, then global.
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.unit_registry.get(hash)
            .or_else(|| self.global_registry.get(hash))
    }

    /// Resolve a type name to its hash.
    ///
    /// Searches in order:
    /// 1. Current namespace (in unit registry, then global)
    /// 2. Imported namespaces (in unit registry, then global)
    /// 3. Global namespace (in unit registry, then global)
    ///
    /// **Performance**: Uses allocation-free hash computation via
    /// `TypeHash::from_qualified_ident()` and cached namespace strings.
    pub fn resolve_type(&self, name: &str) -> Option<TypeHash> {
        // Helper to check both registries by hash
        let lookup_hash = |hash: TypeHash| -> Option<TypeHash> {
            if self.unit_registry.get(hash).is_some() || self.global_registry.get(hash).is_some() {
                Some(hash)
            } else {
                None
            }
        };

        // 1. Try current namespace + name (NO ALLOCATION - uses cached namespace)
        if !self.current_namespace_cached.is_empty() {
            let hash = TypeHash::from_qualified_name(&self.current_namespace_cached, name);
            if let Some(h) = lookup_hash(hash) {
                return Some(h);
            }
        }

        // 2. Try imported namespaces (NO ALLOCATION - uses cached import strings)
        for import in &self.import_caches {
            let hash = TypeHash::from_qualified_name(import, name);
            if let Some(h) = lookup_hash(hash) {
                return Some(h);
            }
        }

        // 3. Try global/unqualified
        let hash = TypeHash::from_name(name);
        lookup_hash(hash)
    }

    /// Resolve a qualified type path (e.g., ["std", "string"]).
    ///
    /// **Performance**: Uses allocation-free hash via `TypeHash::from_ident_parts()`.
    pub fn resolve_qualified_type(&self, path: &[&str]) -> Option<TypeHash> {
        let hash = TypeHash::from_ident_parts(path);
        if self.unit_registry.get(hash).is_some() || self.global_registry.get(hash).is_some() {
            Some(hash)
        } else {
            None
        }
    }

    // ==========================================================================
    // Function Lookup (Layered: unit first, then global)
    // ==========================================================================

    /// Get a function by hash. Checks unit registry first, then global.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.unit_registry.get_function(hash)
            .or_else(|| self.global_registry.get_function(hash))
    }

    /// Get function overloads from both registries.
    pub fn get_function_overloads(&self, name: &str) -> Vec<&FunctionEntry> {
        let mut overloads: Vec<_> = self.unit_registry.get_function_overloads(name).collect();
        overloads.extend(self.global_registry.get_function_overloads(name));
        overloads
    }

    /// Find all functions with the given name (for overload resolution).
    /// Considers current namespace and imports.
    ///
    /// **Performance**: Uses cached namespace strings to avoid allocations.
    pub fn find_functions(&self, name: &str) -> Vec<TypeHash> {
        let mut result = Vec::new();

        // Helper to get overloads from both registries
        let get_overloads = |n: &str| -> Vec<TypeHash> {
            let mut hashes = Vec::new();
            hashes.extend(self.unit_registry.get_function_overloads(n).map(|f| f.def.func_hash));
            hashes.extend(self.global_registry.get_function_overloads(n).map(|f| f.def.func_hash));
            hashes
        };

        // Check current namespace (uses cached string)
        if !self.current_namespace_cached.is_empty() {
            let qualified = format!("{}::{}", self.current_namespace_cached, name);
            result.extend(get_overloads(&qualified));
        }

        // Check imports (uses cached import strings)
        for import in &self.import_caches {
            let qualified = format!("{}::{}", import, name);
            result.extend(get_overloads(&qualified));
        }

        // Check global
        result.extend(get_overloads(name));

        result
    }

    /// Find methods on a type. Checks both registries.
    pub fn find_methods(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        let mut methods = Vec::new();

        // Get methods from unit registry
        if let Some(class) = self.unit_registry.get(type_hash).and_then(|e| e.as_class()) {
            for method_hash in &class.methods {
                if let Some(func) = self.get_function(*method_hash) {
                    if func.def.name == name {
                        methods.push(*method_hash);
                    }
                }
            }
        }

        // Get methods from global registry
        if let Some(class) = self.global_registry.get(type_hash).and_then(|e| e.as_class()) {
            for method_hash in &class.methods {
                if let Some(func) = self.get_function(*method_hash) {
                    if func.def.name == name {
                        methods.push(*method_hash);
                    }
                }
            }
        }

        methods
    }

    // ==========================================================================
    // Global Property Lookup (Layered: unit first, then global)
    // ==========================================================================

    /// Get a global property by hash. Checks unit registry first, then global.
    pub fn get_global(&self, hash: TypeHash) -> Option<&GlobalPropertyEntry> {
        self.unit_registry.get_global(hash)
            .or_else(|| self.global_registry.get_global(hash))
    }

    /// Resolve a global property name to its entry.
    /// Searches: current namespace → imports → global namespace.
    /// Each search checks unit registry first, then global.
    ///
    /// **Performance**: Uses allocation-free hash computation via
    /// `TypeHash::from_ident()` and `TypeHash::from_qualified_ident()`.
    /// Namespace strings are pre-cached to avoid repeated allocations.
    pub fn resolve_global(&self, name: &str) -> Option<&GlobalPropertyEntry> {
        // 1. Try current namespace + name (NO ALLOCATION - uses cached namespace)
        if !self.current_namespace_cached.is_empty() {
            let hash = TypeHash::from_qualified_ident(&self.current_namespace_cached, name);
            if let Some(entry) = self.get_global(hash) {
                return Some(entry);
            }
        }

        // 2. Try imported namespaces (NO ALLOCATION - uses cached import strings)
        for import in &self.import_caches {
            let hash = TypeHash::from_qualified_ident(import, name);
            if let Some(entry) = self.get_global(hash) {
                return Some(entry);
            }
        }

        // 3. Try global/unqualified (NO ALLOCATION)
        let hash = TypeHash::from_ident(name);
        self.get_global(hash)
    }

    // ==========================================================================
    // Namespace Management
    // ==========================================================================

    /// Get current namespace as string (returns cached value).
    pub fn current_namespace(&self) -> &str {
        &self.current_namespace_cached
    }

    /// Enter a namespace. Updates the cached namespace string.
    pub fn enter_namespace(&mut self, name: &str) {
        self.namespace_stack.push(name.to_string());
        self.update_namespace_cache();
    }

    /// Exit current namespace. Updates the cached namespace string.
    pub fn exit_namespace(&mut self) {
        self.namespace_stack.pop();
        self.update_namespace_cache();
    }

    /// Update the cached namespace string (called on enter/exit).
    fn update_namespace_cache(&mut self) {
        self.current_namespace_cached = self.namespace_stack.join("::");
    }

    /// Add an import (using declaration). Updates the import cache.
    pub fn add_import(&mut self, namespace: Vec<String>) {
        let cached = namespace.join("::");
        self.import_caches.push(cached);
        self.imports.push(namespace);
    }

    /// Clear imports (when leaving a scope).
    pub fn clear_imports(&mut self) {
        self.imports.clear();
        self.import_caches.clear();
    }

    // ==========================================================================
    // Registration (Pass 1) - Types go into unit_registry
    // ==========================================================================

    /// Register a script type (non-shared) into the unit registry.
    pub fn register_script_type(&mut self, entry: TypeEntry) -> Result<()> {
        self.unit_registry.register_type(entry)
    }

    /// Register a script function into the unit registry.
    pub fn register_script_function(&mut self, entry: FunctionEntry) -> Result<()> {
        self.unit_registry.register_function(entry)
    }

    /// Register a script global variable into the unit registry.
    pub fn register_script_global(&mut self, entry: GlobalPropertyEntry) -> Result<()> {
        self.unit_registry.register_global(entry)
    }

    /// Register a shared type into the global registry.
    /// Note: This requires global_registry to support interior mutability.
    pub fn register_shared_type(&self, entry: TypeEntry) -> Result<()> {
        self.global_registry.register_shared_type(entry)
    }

    // ==========================================================================
    // Template Instantiation (delegates to TemplateInstantiator - Task 35)
    // ==========================================================================

    /// Instantiate a template type. Template instances go into global registry.
    /// See Task 35: TemplateInstantiator for the actual implementation.
    pub fn instantiate_template(
        &self,
        template_hash: TypeHash,
        type_args: &[TypeHash],
    ) -> Result<TypeHash> {
        // Delegate to TemplateInstantiator (defined in Task 35)
        use crate::template::TemplateInstantiator;
        let instantiator = TemplateInstantiator::new(self.global_registry);
        instantiator.instantiate_type(template_hash, type_args)
    }

    // ==========================================================================
    // Error Handling
    // ==========================================================================

    /// Report an error.
    pub fn error(&mut self, error: CompileError) {
        self.errors.push(error);
    }

    /// Check if there are errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Take collected errors.
    pub fn take_errors(&mut self) -> Vec<CompileError> {
        std::mem::take(&mut self.errors)
    }

    // ==========================================================================
    // Registry Access
    // ==========================================================================

    /// Get the current unit ID.
    pub fn unit_id(&self) -> UnitId {
        self.unit_id
    }

    /// Get mutable access to the unit registry.
    pub fn unit_registry_mut(&mut self) -> &mut TypeRegistry {
        &mut self.unit_registry
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{TypeEntry, ClassEntry, TypeKind, TypeSource};
    use angelscript_registry::TypeRegistry;

    #[test]
    fn context_resolves_primitives_from_global() {
        let mut global = TypeRegistry::new();
        global.register_primitives();

        let ctx = CompilationContext::new(&global, UnitId::new(0));

        // Primitives are in global registry
        assert!(ctx.resolve_type("int").is_some());
        assert!(ctx.resolve_type("float").is_some());
        assert!(ctx.resolve_type("bool").is_some());
    }

    #[test]
    fn context_layered_lookup_unit_first() {
        let global = TypeRegistry::new();
        let mut ctx = CompilationContext::new(&global, UnitId::new(0));

        // Register a type in unit registry
        let hash = TypeHash::from_name("Player");
        let entry = TypeEntry::Class(ClassEntry::new(
            "Player",
            "Player",
            hash,
            TypeKind::script_object(),
            TypeSource::script(UnitId::new(0), Span::default()),
        ));
        ctx.register_script_type(entry).unwrap();

        // Should find in unit registry
        assert!(ctx.get_type(hash).is_some());
        assert!(ctx.resolve_type("Player").is_some());
    }

    #[test]
    fn context_finds_ffi_types_in_global() {
        let mut global = TypeRegistry::new();
        let hash = TypeHash::from_name("Vector3");
        global.register_type(TypeEntry::Class(ClassEntry::new(
            "Vector3",
            "Vector3",
            hash,
            TypeKind::pod::<[f32; 3]>(),
            TypeSource::ffi_untyped(),
        ))).unwrap();

        let ctx = CompilationContext::new(&global, UnitId::new(0));

        // Should find FFI type in global registry
        assert!(ctx.get_type(hash).is_some());
        assert!(ctx.resolve_type("Vector3").is_some());
    }

    #[test]
    fn context_function_overloads_from_both_registries() {
        let mut global = TypeRegistry::new();
        // Add a global function to FFI registry
        // ... (would add FunctionEntry)

        let ctx = CompilationContext::new(&global, UnitId::new(0));

        // get_function_overloads should combine both registries
        let overloads = ctx.get_function_overloads("print");
        // Test that overloads are collected from both
    }

    #[test]
    fn context_memory_cleanup() {
        let global = TypeRegistry::new();
        let mut ctx = CompilationContext::new(&global, UnitId::new(0));

        // Register many types in unit registry
        for i in 0..100 {
            let hash = TypeHash::from_name(&format!("Type{}", i));
            let entry = TypeEntry::Class(ClassEntry::new(
                &format!("Type{}", i),
                &format!("Type{}", i),
                hash,
                TypeKind::script_object(),
                TypeSource::script(UnitId::new(0), Span::default()),
            ));
            ctx.register_script_type(entry).unwrap();
        }

        // Drop ctx - unit_registry should be freed
        drop(ctx);
        // global registry still exists and is unchanged
    }
}
```

## Prerequisites: Allocation-Free TypeHash Methods

This task requires adding allocation-free hash methods to `TypeHash` in `angelscript-core/src/type_hash.rs`:

```rust
impl TypeHash {
    /// Create an identifier hash from a single name (no allocation).
    /// Uses IDENT domain constant for global property lookups.
    #[inline]
    pub fn from_ident(name: &str) -> Self {
        TypeHash(hash_constants::IDENT ^ xxh64(name.as_bytes(), 0))
    }

    /// Create a qualified name hash WITHOUT allocation.
    /// Computes same hash as from_name("namespace::name") but without format!().
    /// For type lookups - uses TYPE domain constant.
    #[inline]
    pub fn from_qualified_name(namespace: &str, name: &str) -> Self {
        // Build "namespace::name" hash incrementally
        let mut hasher_state = 0u64;
        hasher_state = xxh64(namespace.as_bytes(), hasher_state);
        hasher_state = xxh64(b"::", hasher_state);
        hasher_state = xxh64(name.as_bytes(), hasher_state);
        TypeHash(hash_constants::TYPE ^ hasher_state)
    }

    /// Create a qualified identifier hash WITHOUT allocation.
    /// For global property lookups - uses IDENT domain constant.
    #[inline]
    pub fn from_qualified_ident(namespace: &str, name: &str) -> Self {
        let mut hasher_state = 0u64;
        hasher_state = xxh64(namespace.as_bytes(), hasher_state);
        hasher_state = xxh64(b"::", hasher_state);
        hasher_state = xxh64(name.as_bytes(), hasher_state);
        TypeHash(hash_constants::IDENT ^ hasher_state)
    }

    /// Create an identifier hash from multiple path parts WITHOUT allocation.
    /// from_ident_parts(&["std", "math", "PI"]) == from_ident("std::math::PI")
    #[inline]
    pub fn from_ident_parts(parts: &[&str]) -> Self {
        if parts.is_empty() {
            return TypeHash::EMPTY;
        }
        if parts.len() == 1 {
            return Self::from_name(parts[0]);
        }

        let mut hasher_state = 0u64;
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                hasher_state = xxh64(b"::", hasher_state);
            }
            hasher_state = xxh64(part.as_bytes(), hasher_state);
        }
        TypeHash(hash_constants::TYPE ^ hasher_state)
    }
}
```

**Note**: The exact hash computation must match what `TypeHash::from_name()` would produce for the equivalent string. Unit tests should verify:
- `TypeHash::from_qualified_name("ns", "Foo")` == `TypeHash::from_name("ns::Foo")`
- `TypeHash::from_ident_parts(&["a", "b", "c"])` == `TypeHash::from_name("a::b::c")`

## Acceptance Criteria

- [ ] CompilationContext uses layered lookup (unit → global)
- [ ] Type lookup works: unit registry first, then global
- [ ] Function lookup works: unit registry first, then global
- [ ] get_function_overloads combines both registries
- [ ] Namespace resolution searches both registries
- [ ] Script types go into unit_registry
- [ ] Shared types go into global_registry
- [ ] Template instantiation delegates to TemplateInstantiator
- [ ] Error collection works
- [ ] Memory is released when unit_registry is dropped
- [ ] **Global property lookup uses allocation-free hash computation**
- [ ] **Namespace caching avoids repeated join("::") allocations**
- [ ] **`register_script_global()` works for script-declared globals**
- [ ] All tests pass

## Next Phase

Task 34: Type Resolution - convert TypeExpr AST to DataType
