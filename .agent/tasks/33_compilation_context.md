# Task 33: Compilation Context (Revised)

## Overview

Provide the **building blocks** for namespace-aware type resolution with O(1) lookup performance. This task creates the `CompilationContext` that other passes (Registration, Compilation) will use.

## Key Design Decision

**Materialized Scope View** - We build a cached view of all accessible symbols when entering a namespace or adding imports, enabling O(1) resolution instead of O(m) namespace iteration.

**Rationale:** Type resolutions are frequent (hundreds per namespace block), namespace changes are infrequent (few per file). O(1) resolution is worth the O(t) rebuild cost.

## Goals

1. Add namespace-partitioned indexes to SymbolRegistry
2. Create `Scope` struct (materialized view of accessible symbols)
3. Create `CompilationContext` with layered registries and scope management
4. Implement O(1) resolution methods
5. Implement ambiguity detection during scope building

## Dependencies

- Task 31: Compiler Foundation (SymbolRegistry exists)
- angelscript-core types (TypeHash, DataType, TypeEntry)

## Architecture

```
                  ┌─────────────────────────┐
                  │    CompilationContext   │
                  │  ┌───────────────────┐  │
                  │  │      Scope        │  │  ← Materialized view
                  │  │  (O(1) lookups)   │  │    rebuilt on ns change
                  │  └───────────────────┘  │
                  │           │             │
                  │     ┌─────┴─────┐       │
                  │     ▼           ▼       │
┌─────────────────┴─────────┐ ┌────────────┴───────────────┐
│  global_registry (ref)    │ │  unit_registry (owned)     │
│  - FFI types/functions    │ │  - Script types/functions  │
│  - Template instances     │ │  - Local to compilation    │
│  + types_by_namespace     │ │  + types_by_namespace      │
└───────────────────────────┘ └────────────────────────────┘
```

## Files to Create/Modify

```
crates/angelscript-registry/src/
├── registry.rs           # Add namespace indexes
└── lib.rs

crates/angelscript-compiler/src/
├── context.rs            # NEW: CompilationContext + Scope
├── script_defs.rs        # NEW: ScriptTypeDef, ScriptFunctionDef
└── lib.rs                # Export new modules
```

## Detailed Implementation

### Phase 1: Registry Namespace Indexes (registry.rs)

Add efficient namespace iteration to SymbolRegistry:

```rust
use rustc_hash::FxHashMap;
use angelscript_core::TypeHash;

pub struct SymbolRegistry {
    // Existing fields
    types: FxHashMap<TypeHash, TypeEntry>,
    functions: FxHashMap<TypeHash, FunctionEntry>,
    globals: FxHashMap<TypeHash, GlobalPropertyEntry>,
    type_by_name: FxHashMap<String, TypeHash>,

    // NEW: Namespace-partitioned indexes for efficient iteration
    // namespace -> (simple_name -> hash)
    types_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,
    functions_by_namespace: FxHashMap<String, FxHashMap<String, Vec<TypeHash>>>,
    globals_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,
}

impl SymbolRegistry {
    /// Get all types in a namespace as a map of simple_name -> hash.
    pub fn get_namespace_types(&self, ns: &str) -> Option<&FxHashMap<String, TypeHash>> {
        self.types_by_namespace.get(ns)
    }

    /// Get all functions in a namespace as a map of simple_name -> hashes.
    pub fn get_namespace_functions(&self, ns: &str) -> Option<&FxHashMap<String, Vec<TypeHash>>> {
        self.functions_by_namespace.get(ns)
    }

    /// Get all globals in a namespace as a map of simple_name -> hash.
    pub fn get_namespace_globals(&self, ns: &str) -> Option<&FxHashMap<String, TypeHash>> {
        self.globals_by_namespace.get(ns)
    }

    /// Register a type (update to populate namespace index).
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let hash = entry.type_hash();
        let qualified_name = entry.qualified_name().to_string();

        // Existing registration
        if self.types.contains_key(&hash) {
            return Err(RegistrationError::DuplicateType(qualified_name));
        }
        self.type_by_name.insert(qualified_name.clone(), hash);
        self.types.insert(hash, entry);

        // NEW: Add to namespace index
        let (ns, simple) = split_qualified_name(&qualified_name);
        self.types_by_namespace
            .entry(ns.to_string())
            .or_default()
            .insert(simple.to_string(), hash);

        Ok(())
    }

    /// Register a function (update to populate namespace index).
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        let hash = entry.hash();
        let qualified_name = entry.def().qualified_name().to_string();

        // Existing registration
        if self.functions.contains_key(&hash) {
            return Err(RegistrationError::DuplicateFunction(qualified_name));
        }
        self.functions.insert(hash, entry);

        // NEW: Add to namespace index (functions can overload)
        let (ns, simple) = split_qualified_name(&qualified_name);
        self.functions_by_namespace
            .entry(ns.to_string())
            .or_default()
            .entry(simple.to_string())
            .or_default()
            .push(hash);

        Ok(())
    }

    /// Register a global (update to populate namespace index).
    pub fn register_global(&mut self, entry: GlobalPropertyEntry) -> Result<(), RegistrationError> {
        let hash = entry.hash();
        let qualified_name = entry.qualified_name().to_string();

        // Existing registration
        if self.globals.contains_key(&hash) {
            return Err(RegistrationError::DuplicateGlobal(qualified_name));
        }
        self.globals.insert(hash, entry);

        // NEW: Add to namespace index
        let (ns, simple) = split_qualified_name(&qualified_name);
        self.globals_by_namespace
            .entry(ns.to_string())
            .or_default()
            .insert(simple.to_string(), hash);

        Ok(())
    }
}

/// Split "Game::Entities::Player" into ("Game::Entities", "Player").
fn split_qualified_name(qualified: &str) -> (&str, &str) {
    match qualified.rsplit_once("::") {
        Some((ns, simple)) => (ns, simple),
        None => ("", qualified),  // Global namespace
    }
}
```

### Phase 2: Scope Struct (context.rs)

```rust
use rustc_hash::FxHashMap;
use angelscript_core::TypeHash;

/// Materialized view of symbols accessible without qualification.
/// Rebuilt when namespace changes or imports are added.
#[derive(Default)]
pub struct Scope {
    /// "Player" -> TypeHash of qualified "Game::Entities::Player"
    pub types: FxHashMap<String, TypeHash>,

    /// "print" -> [hash1, hash2] (multiple overloads from different namespaces)
    pub functions: FxHashMap<String, Vec<TypeHash>>,

    /// "GRAVITY" -> TypeHash of "Physics::GRAVITY"
    pub globals: FxHashMap<String, TypeHash>,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.types.clear();
        self.functions.clear();
        self.globals.clear();
    }
}
```

### Phase 3: CompilationContext (context.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash, CompilationError};
use angelscript_registry::SymbolRegistry;

/// Compilation context with layered registries and namespace-aware resolution.
///
/// ## Design: Materialized Scope View
///
/// Instead of O(m) iteration through namespaces on each lookup, we maintain a
/// `Scope` that is a materialized view of all accessible symbols. This is rebuilt
/// when namespace changes occur (enter/exit namespace, add import).
///
/// **Complexity:**
/// - `resolve_type()`: O(1) - single HashMap lookup
/// - `enter_namespace()`: O(t) - rebuilds scope where t = total accessible types
/// - Namespace changes are infrequent, resolutions are frequent, so this is optimal.
pub struct CompilationContext<'a> {
    /// Global registry (FFI types, shared types)
    global_registry: &'a SymbolRegistry,

    /// Unit-local registry (script types being compiled)
    unit_registry: SymbolRegistry,

    /// Materialized scope for O(1) resolution
    scope: Scope,

    /// Namespace stack for current position
    namespace_stack: Vec<String>,

    /// Active using namespace imports
    imports: Vec<String>,

    /// Errors collected during compilation
    errors: Vec<CompilationError>,
}

impl<'a> CompilationContext<'a> {
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

    // ==========================================================================
    // Namespace Management
    // ==========================================================================

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

    // ==========================================================================
    // Scope Building (O(t) where t = total accessible types)
    // ==========================================================================

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
        // Add types from unit registry
        if let Some(types) = self.unit_registry.get_namespace_types(ns) {
            for (simple, &hash) in types {
                self.add_type_to_scope(simple, hash, ns);
            }
        }

        // Add types from global registry
        if let Some(types) = self.global_registry.get_namespace_types(ns) {
            for (simple, &hash) in types {
                self.add_type_to_scope(simple, hash, ns);
            }
        }

        // Add functions from unit registry
        if let Some(funcs) = self.unit_registry.get_namespace_functions(ns) {
            for (simple, hashes) in funcs {
                for &hash in hashes {
                    self.add_function_to_scope(simple, hash);
                }
            }
        }

        // Add functions from global registry
        if let Some(funcs) = self.global_registry.get_namespace_functions(ns) {
            for (simple, hashes) in funcs {
                for &hash in hashes {
                    self.add_function_to_scope(simple, hash);
                }
            }
        }

        // Add globals from unit registry
        if let Some(globals) = self.unit_registry.get_namespace_globals(ns) {
            for (simple, &hash) in globals {
                self.add_global_to_scope(simple, hash, ns);
            }
        }

        // Add globals from global registry
        if let Some(globals) = self.global_registry.get_namespace_globals(ns) {
            for (simple, &hash) in globals {
                self.add_global_to_scope(simple, hash, ns);
            }
        }
    }

    fn add_type_to_scope(&mut self, simple: &str, hash: TypeHash, ns: &str) {
        if let Some(&existing) = self.scope.types.get(simple) {
            if existing != hash {
                // Ambiguity - only error if both are from imports (not current ns shadowing)
                let current = self.current_namespace();
                if ns != current {
                    self.errors.push(CompilationError::AmbiguousType {
                        name: simple.to_string(),
                        candidates: vec![existing, hash],
                        span: Span::default(),
                    });
                }
            }
        }
        // Later additions (current namespace) shadow earlier ones (imports)
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
        if let Some(&existing) = self.scope.globals.get(simple) {
            if existing != hash {
                let current = self.current_namespace();
                if ns != current {
                    self.errors.push(CompilationError::AmbiguousGlobal {
                        name: simple.to_string(),
                        candidates: vec![existing, hash],
                        span: Span::default(),
                    });
                }
            }
        }
        self.scope.globals.insert(simple.to_string(), hash);
    }

    // ==========================================================================
    // Resolution Methods (O(1))
    // ==========================================================================

    /// Resolve a type name to its hash. O(1) for unqualified, O(1) for qualified.
    pub fn resolve_type(&self, name: &str) -> Option<TypeHash> {
        if name.contains("::") {
            // Qualified name: bypass scope, direct registry lookup
            let hash = TypeHash::from_name(name);
            if self.unit_registry.get(hash).is_some()
                || self.global_registry.get(hash).is_some() {
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
            // Qualified function lookup - would need additional handling
            return None;
        }

        self.scope.functions.get(name).map(|v| v.as_slice())
    }

    /// Resolve a global variable name to its hash. O(1).
    pub fn resolve_global(&self, name: &str) -> Option<TypeHash> {
        if name.contains("::") {
            let hash = TypeHash::from_name(name);
            if self.unit_registry.get_global(hash).is_some()
                || self.global_registry.get_global(hash).is_some() {
                return Some(hash);
            }
            return None;
        }

        self.scope.globals.get(name).copied()
    }

    // ==========================================================================
    // Direct Registry Access (by hash) - for after resolution
    // ==========================================================================

    /// Get a type entry by hash (layered lookup).
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.unit_registry.get(hash)
            .or_else(|| self.global_registry.get(hash))
    }

    /// Get a function entry by hash (layered lookup).
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.unit_registry.get_function(hash)
            .or_else(|| self.global_registry.get_function(hash))
    }

    /// Get a global entry by hash (layered lookup).
    pub fn get_global_entry(&self, hash: TypeHash) -> Option<&GlobalPropertyEntry> {
        self.unit_registry.get_global(hash)
            .or_else(|| self.global_registry.get_global(hash))
    }

    /// Find methods on a type by name.
    pub fn find_methods(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        let mut methods = Vec::new();

        // Check type in unit registry
        if let Some(class) = self.unit_registry.get(type_hash).and_then(|e| e.as_class()) {
            for &method_hash in &class.methods {
                if let Some(func) = self.get_function(method_hash) {
                    if func.def().name == name {
                        methods.push(method_hash);
                    }
                }
            }
        }

        // Check type in global registry
        if let Some(class) = self.global_registry.get(type_hash).and_then(|e| e.as_class()) {
            for &method_hash in &class.methods {
                if let Some(func) = self.get_function(method_hash) {
                    if func.def().name == name {
                        methods.push(method_hash);
                    }
                }
            }
        }

        methods
    }

    // ==========================================================================
    // Registration (for unit registry)
    // ==========================================================================

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

    // ==========================================================================
    // Error Handling
    // ==========================================================================

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

    /// Get mutable unit registry for direct manipulation.
    pub fn unit_registry_mut(&mut self) -> &mut SymbolRegistry {
        &mut self.unit_registry
    }

    /// Get unit registry.
    pub fn unit_registry(&self) -> &SymbolRegistry {
        &self.unit_registry
    }
}
```

### Phase 4: Script Definitions (script_defs.rs)

Pending type definitions for script types discovered during registration:

```rust
use angelscript_core::{DataType, TypeHash, Visibility, Span};
use angelscript_parser::ast::{ClassDecl, FunctionDecl, EnumDecl};

/// A script type definition discovered during registration.
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

/// A script function definition discovered during registration.
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
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ScriptParam {
    pub name: String,
    pub data_type: DataType,
    pub has_default: bool,
}
```

## Complexity Analysis

| Operation | Time Complexity | Notes |
|-----------|-----------------|-------|
| `enter_namespace()` | O(t) | t = total types in accessible namespaces |
| `exit_namespace()` | O(t) | Rebuild required |
| `add_import()` | O(t) | Rebuild required |
| **`resolve_type()`** | **O(1)** | Single HashMap lookup |
| **`resolve_function()`** | **O(1)** | Single HashMap lookup |
| **`resolve_global()`** | **O(1)** | Single HashMap lookup |
| `register_type()` | O(t) | Triggers rebuild |

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_type_global_namespace() {
        let global = SymbolRegistry::new();
        // Register "int" type in global namespace
        let mut ctx = CompilationContext::new(&global);
        assert!(ctx.resolve_type("int").is_some());
    }

    #[test]
    fn resolve_type_current_namespace() {
        let mut global = SymbolRegistry::new();
        // Register "Game::Player" type
        let mut ctx = CompilationContext::new(&global);
        ctx.enter_namespace("Game");
        // "Player" should resolve to "Game::Player"
        assert!(ctx.resolve_type("Player").is_some());
    }

    #[test]
    fn resolve_type_imported_namespace() {
        let mut global = SymbolRegistry::new();
        // Register "Utils::Helper" type
        let mut ctx = CompilationContext::new(&global);
        ctx.add_import("Utils");
        // "Helper" should resolve to "Utils::Helper"
        assert!(ctx.resolve_type("Helper").is_some());
    }

    #[test]
    fn resolve_type_qualified() {
        let mut global = SymbolRegistry::new();
        // Register "Game::Player" type
        let ctx = CompilationContext::new(&global);
        // "Game::Player" should resolve directly
        assert!(ctx.resolve_type("Game::Player").is_some());
    }

    #[test]
    fn resolve_type_shadowing() {
        // Current namespace shadows imports
        let mut global = SymbolRegistry::new();
        // Register both "Utils::Player" and "Game::Player"
        let mut ctx = CompilationContext::new(&global);
        ctx.add_import("Utils");
        ctx.enter_namespace("Game");
        // "Player" should resolve to "Game::Player" (current shadows import)
        let hash = ctx.resolve_type("Player").unwrap();
        assert_eq!(hash, TypeHash::from_name("Game::Player"));
    }

    #[test]
    fn resolve_type_ambiguous() {
        let mut global = SymbolRegistry::new();
        // Register "A::Player" and "B::Player"
        let mut ctx = CompilationContext::new(&global);
        ctx.add_import("A");
        ctx.add_import("B");
        // Ambiguity error should be recorded
        assert!(ctx.has_errors());
    }

    #[test]
    fn resolve_function_overloads() {
        let mut global = SymbolRegistry::new();
        // Register print(int) and print(string) in global namespace
        let ctx = CompilationContext::new(&global);
        let overloads = ctx.resolve_function("print");
        assert_eq!(overloads.map(|v| v.len()), Some(2));
    }

    #[test]
    fn namespace_stack() {
        let global = SymbolRegistry::new();
        let mut ctx = CompilationContext::new(&global);

        ctx.enter_namespace("A");
        assert_eq!(ctx.current_namespace(), "A");

        ctx.enter_namespace("B");
        assert_eq!(ctx.current_namespace(), "A::B");

        ctx.exit_namespace();
        assert_eq!(ctx.current_namespace(), "A");
    }

    #[test]
    fn layered_registry_lookup() {
        let mut global = SymbolRegistry::new();
        // Register FFI type "Vector3" in global registry

        let mut ctx = CompilationContext::new(&global);
        // Register script type "Player" in unit registry

        // Both should be accessible
        assert!(ctx.resolve_type("Vector3").is_some());  // from global
        assert!(ctx.resolve_type("Player").is_some());   // from unit
    }
}
```

## Acceptance Criteria

- [ ] SymbolRegistry has namespace-partitioned indexes
- [ ] `get_namespace_types()` returns O(1) map access
- [ ] Scope struct stores materialized name -> hash mappings
- [ ] CompilationContext manages layered registries (unit + global)
- [ ] `enter_namespace()` / `exit_namespace()` rebuild scope
- [ ] `add_import()` adds namespace to scope
- [ ] `resolve_type()` is O(1) for unqualified names
- [ ] `resolve_function()` returns all overloads
- [ ] `resolve_global()` is O(1)
- [ ] Qualified names bypass scope and use direct lookup
- [ ] Ambiguity detected during scope build (not per-lookup)
- [ ] Current namespace shadows imports (build order)
- [ ] Layered lookup: unit registry first, then global
- [ ] All tests pass

## What Goes in Other Tasks

| Task | Uses From Task 33 |
|------|-------------------|
| **Task 34** (Type Resolution) | `ctx.resolve_type()` for TypeExpr AST |
| **Task 38** (Registration Pass) | `ctx.enter_namespace()`, `ctx.register_*()` |
| **Tasks 41-44** (Expression/Statement) | `ctx.resolve_type()`, `ctx.resolve_function()`, `ctx.resolve_global()` |
| **Task 46** (Compilation Pass) | `ctx.enter_namespace()`, `ctx.exit_namespace()` |

## Next Task

Task 34: Type Resolution - Using CompilationContext to resolve TypeExpr AST nodes
