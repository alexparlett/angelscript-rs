# Task 32: Compilation Context

## Overview

Create the `CompilationContext` that wraps `TypeRegistry` and provides unified lookup for both FFI and script-defined types/functions during compilation.

## Goals

1. Unified lookup across FFI registry and script definitions
2. Namespace management (current namespace, imports)
3. Template instance caching
4. Error collection during compilation

## Dependencies

- Task 31: Compiler Foundation

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
use angelscript_core::{DataType, Span, TypeEntry, TypeHash, UnitId};
use angelscript_registry::TypeRegistry;
use rustc_hash::FxHashMap;

use crate::error::{CompileError, Result};
use crate::script_defs::{ScriptFunctionDef, ScriptTypeDef};

/// Compilation context providing unified type/function lookup.
///
/// Wraps the FFI TypeRegistry and adds script definitions during compilation.
pub struct CompilationContext<'reg> {
    /// The unified type registry (FFI types)
    registry: &'reg mut TypeRegistry,

    /// Current compilation unit
    unit_id: UnitId,

    /// Script-defined types (built during registration pass)
    script_types: FxHashMap<TypeHash, ScriptTypeDef>,

    /// Script-defined functions (built during registration pass)
    script_functions: FxHashMap<TypeHash, ScriptFunctionDef>,

    /// Type name -> TypeHash lookup (includes both FFI and script)
    types_by_name: FxHashMap<String, TypeHash>,

    /// Function name -> Vec<TypeHash> for overloaded functions
    functions_by_name: FxHashMap<String, Vec<TypeHash>>,

    /// Template instance cache
    template_cache: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,

    /// Current namespace path during compilation
    namespace_stack: Vec<String>,

    /// Imported namespaces (from 'using' declarations)
    imports: Vec<Vec<String>>,

    /// Collected errors
    errors: Vec<CompileError>,
}

impl<'reg> CompilationContext<'reg> {
    /// Create a new compilation context.
    pub fn new(registry: &'reg mut TypeRegistry, unit_id: UnitId) -> Self {
        // Pre-populate types_by_name from registry
        let mut types_by_name = FxHashMap::default();
        for (hash, entry) in registry.iter_types() {
            let name = match entry {
                TypeEntry::Primitive(p) => p.name.clone(),
                TypeEntry::Class(c) => c.qualified_name.clone(),
                TypeEntry::Enum(e) => e.qualified_name.clone(),
                TypeEntry::Interface(i) => i.qualified_name.clone(),
                TypeEntry::Funcdef(f) => f.qualified_name.clone(),
                TypeEntry::TemplateParam(t) => t.qualified_name.clone(),
            };
            types_by_name.insert(name, *hash);
        }

        // Pre-populate functions_by_name from registry
        let mut functions_by_name: FxHashMap<String, Vec<TypeHash>> = FxHashMap::default();
        for (hash, func) in registry.iter_functions() {
            functions_by_name
                .entry(func.def.name.clone())
                .or_default()
                .push(*hash);
            // Also add qualified name
            if !func.def.namespace.is_empty() {
                let qualified = format!("{}::{}", func.def.namespace.join("::"), func.def.name);
                functions_by_name.entry(qualified).or_default().push(*hash);
            }
        }

        // Pre-populate template cache from registry
        let template_cache = registry.get_template_cache().clone();

        Self {
            registry,
            unit_id,
            script_types: FxHashMap::default(),
            script_functions: FxHashMap::default(),
            types_by_name,
            functions_by_name,
            template_cache,
            namespace_stack: Vec::new(),
            imports: Vec::new(),
            errors: Vec::new(),
        }
    }

    // ==========================================================================
    // Type Lookup
    // ==========================================================================

    /// Get a type entry by hash (checks both FFI and script).
    pub fn get_type(&self, hash: TypeHash) -> Option<TypeRef<'_>> {
        // Check FFI registry first
        if let Some(entry) = self.registry.get(hash) {
            return Some(TypeRef::Ffi(entry));
        }
        // Check script definitions
        if let Some(def) = self.script_types.get(&hash) {
            return Some(TypeRef::Script(def));
        }
        None
    }

    /// Resolve a type name to its hash.
    ///
    /// Searches in order:
    /// 1. Current namespace
    /// 2. Imported namespaces
    /// 3. Global namespace
    pub fn resolve_type(&self, name: &str) -> Option<TypeHash> {
        // 1. Try current namespace + name
        if !self.namespace_stack.is_empty() {
            let qualified = format!("{}::{}", self.current_namespace(), name);
            if let Some(hash) = self.types_by_name.get(&qualified) {
                return Some(*hash);
            }
        }

        // 2. Try imported namespaces
        for import in &self.imports {
            let qualified = format!("{}::{}", import.join("::"), name);
            if let Some(hash) = self.types_by_name.get(&qualified) {
                return Some(*hash);
            }
        }

        // 3. Try global/unqualified
        self.types_by_name.get(name).copied()
    }

    /// Resolve a qualified type path (e.g., ["std", "string"]).
    pub fn resolve_qualified_type(&self, path: &[&str]) -> Option<TypeHash> {
        let qualified = path.join("::");
        self.types_by_name.get(&qualified).copied()
    }

    // ==========================================================================
    // Function Lookup
    // ==========================================================================

    /// Get a function by hash.
    pub fn get_function(&self, hash: TypeHash) -> Option<FunctionRef<'_>> {
        if let Some(entry) = self.registry.get_function(hash) {
            return Some(FunctionRef::Ffi(entry));
        }
        if let Some(def) = self.script_functions.get(&hash) {
            return Some(FunctionRef::Script(def));
        }
        None
    }

    /// Find all functions with the given name (for overload resolution).
    pub fn find_functions(&self, name: &str) -> Vec<TypeHash> {
        let mut result = Vec::new();

        // Check current namespace
        if !self.namespace_stack.is_empty() {
            let qualified = format!("{}::{}", self.current_namespace(), name);
            if let Some(hashes) = self.functions_by_name.get(&qualified) {
                result.extend(hashes);
            }
        }

        // Check imports
        for import in &self.imports {
            let qualified = format!("{}::{}", import.join("::"), name);
            if let Some(hashes) = self.functions_by_name.get(&qualified) {
                result.extend(hashes);
            }
        }

        // Check global
        if let Some(hashes) = self.functions_by_name.get(name) {
            result.extend(hashes);
        }

        result
    }

    /// Find methods on a type.
    pub fn find_methods(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        let mut methods = Vec::new();

        // Get methods from FFI registry
        if let Some(class) = self.registry.get(type_hash).and_then(|e| e.as_class()) {
            for method_hash in &class.methods {
                if let Some(func) = self.registry.get_function(*method_hash) {
                    if func.def.name == name {
                        methods.push(*method_hash);
                    }
                }
            }
        }

        // Get methods from script definitions
        for (hash, func) in &self.script_functions {
            if func.object_type == Some(type_hash) && func.name == name {
                methods.push(*hash);
            }
        }

        methods
    }

    // ==========================================================================
    // Namespace Management
    // ==========================================================================

    /// Get current namespace as string.
    pub fn current_namespace(&self) -> String {
        self.namespace_stack.join("::")
    }

    /// Enter a namespace.
    pub fn enter_namespace(&mut self, name: &str) {
        self.namespace_stack.push(name.to_string());
    }

    /// Exit current namespace.
    pub fn exit_namespace(&mut self) {
        self.namespace_stack.pop();
    }

    /// Add an import (using declaration).
    pub fn add_import(&mut self, namespace: Vec<String>) {
        self.imports.push(namespace);
    }

    /// Clear imports (when leaving a scope).
    pub fn clear_imports(&mut self) {
        self.imports.clear();
    }

    // ==========================================================================
    // Registration (Pass 1)
    // ==========================================================================

    /// Register a script type definition.
    pub fn register_script_type(&mut self, def: ScriptTypeDef) {
        let hash = def.type_hash;
        let name = def.qualified_name.clone();
        self.types_by_name.insert(name, hash);
        self.script_types.insert(hash, def);
    }

    /// Register a script function definition.
    pub fn register_script_function(&mut self, def: ScriptFunctionDef) {
        let hash = def.func_hash;
        let name = def.name.clone();
        self.functions_by_name.entry(name).or_default().push(hash);
        if !def.qualified_name.is_empty() {
            self.functions_by_name
                .entry(def.qualified_name.clone())
                .or_default()
                .push(hash);
        }
        self.script_functions.insert(hash, def);
    }

    // ==========================================================================
    // Template Cache
    // ==========================================================================

    /// Get a cached template instance.
    pub fn get_cached_template(&self, template: TypeHash, args: &[TypeHash]) -> Option<TypeHash> {
        self.template_cache.get(&(template, args.to_vec())).copied()
    }

    /// Cache a template instance.
    pub fn cache_template(&mut self, template: TypeHash, args: Vec<TypeHash>, instance: TypeHash) {
        self.template_cache.insert((template, args), instance);
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

    /// Get mutable access to the registry for type registration.
    pub fn registry_mut(&mut self) -> &mut TypeRegistry {
        self.registry
    }

    /// Get the current unit ID.
    pub fn unit_id(&self) -> UnitId {
        self.unit_id
    }
}

/// Reference to a type (either FFI or script-defined).
pub enum TypeRef<'a> {
    Ffi(&'a TypeEntry),
    Script(&'a ScriptTypeDef),
}

/// Reference to a function (either FFI or script-defined).
pub enum FunctionRef<'a> {
    Ffi(&'a angelscript_core::FunctionEntry),
    Script(&'a ScriptFunctionDef),
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_registry::TypeRegistry;

    #[test]
    fn context_resolves_primitives() {
        let mut registry = TypeRegistry::new();
        registry.register_primitives();

        let ctx = CompilationContext::new(&mut registry, UnitId::new(0));

        assert!(ctx.resolve_type("int").is_some());
        assert!(ctx.resolve_type("float").is_some());
        assert!(ctx.resolve_type("bool").is_some());
    }

    #[test]
    fn context_namespace_resolution() {
        let mut registry = TypeRegistry::new();
        let ctx = CompilationContext::new(&mut registry, UnitId::new(0));

        // Register a type in a namespace
        let def = ScriptTypeDef {
            name: "Player".to_string(),
            qualified_name: "game::Player".to_string(),
            type_hash: TypeHash::from_name("game::Player"),
            kind: ScriptTypeKind::Class {
                base_class: None,
                interfaces: vec![],
                is_final: false,
                is_abstract: false,
            },
            span: Span::default(),
        };
        ctx.register_script_type(def);

        // Should find with qualified name
        assert!(ctx.resolve_qualified_type(&["game", "Player"]).is_some());
    }
}
```

## Acceptance Criteria

- [ ] CompilationContext wraps TypeRegistry correctly
- [ ] Type lookup works for both FFI and script types
- [ ] Function lookup works for both FFI and script functions
- [ ] Namespace resolution follows correct order
- [ ] Template cache integration works
- [ ] Error collection works
- [ ] All tests pass

## Next Phase

Task 33: Type Resolution - convert TypeExpr AST to DataType
