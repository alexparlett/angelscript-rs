//! CompilationContext - unified context for compilation.
//!
//! `CompilationContext` is the unified facade for type/function lookups during compilation.
//! It holds an immutable `Arc<FfiRegistry>` (shared across all Units) and a mutable
//! `ScriptRegistry` (per-compilation), providing unified lookups across both.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   CompilationContext                        │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  type_by_name: HashMap<String, TypeHash>            │   │
//! │  │  func_by_name: HashMap<String, Vec<TypeHash>>       │   │
//! │  │  (unified name lookup - FFI + Script)               │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  ffi: Arc<FfiRegistry>                              │   │
//! │  │  (immutable, shared - primitives, FFI types)        │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  script: ScriptRegistry                             │   │
//! │  │  (mutable - script-defined types)                   │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  namespace_path: Vec<String>                        │   │
//! │  │  imported_namespaces: Vec<String>                   │   │
//! │  │  (namespace tracking for name resolution)           │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Lookup Behavior
//!
//! - **By Hash** (`get_type`, `get_function`): Tries FFI first, then Script
//! - **By Name** (`lookup_type`, `lookup_functions`): Single unified HashMap lookup
//! - **Name Resolution** (`resolve_type`): Checks current namespace, imports, then global
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use angelscript_ffi::FfiRegistry;
//! use angelscript_compiler::CompilationContext;
//!
//! let ffi_registry = Arc::new(FfiRegistryBuilder::new().build().unwrap());
//! let mut ctx = CompilationContext::new(ffi_registry);
//!
//! // Lookup primitives (from FFI)
//! let int_hash = ctx.lookup_type("int").unwrap();
//!
//! // Unified lookup works for both FFI and script types
//! assert!(ctx.lookup_type("int").is_some());
//! ```

use std::sync::Arc;

use rustc_hash::FxHashMap;

use angelscript_ffi::FfiRegistry;
use crate::registry::ScriptRegistry;
use crate::types::{
    DataType, FunctionDef, OperatorBehavior, PropertyAccessors, TypeBehaviors, TypeDef, TypeHash,
    primitives,
};

/// Error during name resolution.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ResolutionError {
    /// Type not found.
    #[error("unknown type '{0}'")]
    UnknownType(String),

    /// Function not found.
    #[error("unknown function '{0}'")]
    UnknownFunction(String),

    /// Ambiguous type reference.
    #[error("ambiguous type '{0}': could be {1}")]
    AmbiguousType(String, String),
}

/// Unified compilation context providing access to both FFI and Script registries.
///
/// This is the primary interface for type and function lookups during compilation.
/// It maintains unified name→hash maps for fast lookup, and routes hash-based queries
/// to the appropriate registry (FFI first, then Script).
pub struct CompilationContext {
    /// Immutable FFI registry (shared across all Units).
    ffi: Arc<FfiRegistry>,

    /// Mutable script registry (per-compilation).
    script: ScriptRegistry,

    /// Unified type name → TypeHash map (FFI + Script).
    type_by_name: FxHashMap<String, TypeHash>,

    /// Unified function name → func_hashes map (FFI + Script).
    func_by_name: FxHashMap<String, Vec<TypeHash>>,

    /// Current namespace path (e.g., ["Game", "Player"]).
    namespace_path: Vec<String>,

    /// Imported namespaces for name resolution.
    imported_namespaces: Vec<String>,
}

impl std::fmt::Debug for CompilationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilationContext")
            .field("ffi", &self.ffi)
            .field("script", &self.script)
            .field("type_by_name", &format!("<{} entries>", self.type_by_name.len()))
            .field("func_by_name", &format!("<{} entries>", self.func_by_name.len()))
            .field("namespace_path", &self.namespace_path)
            .field("imported_namespaces", &self.imported_namespaces)
            .finish()
    }
}

impl CompilationContext {
    /// Create a new compilation context with the given FFI registry.
    ///
    /// The unified name maps are initialized from the FFI registry.
    pub fn new(ffi: Arc<FfiRegistry>) -> Self {
        Self {
            type_by_name: ffi.type_by_name(),
            func_by_name: ffi.func_by_name(),
            ffi,
            script: ScriptRegistry::new(),
            namespace_path: Vec::new(),
            imported_namespaces: Vec::new(),
        }
    }

    /// Get a reference to the underlying FFI registry.
    pub fn ffi(&self) -> &FfiRegistry {
        &self.ffi
    }

    /// Get a reference to the underlying script registry.
    pub fn script(&self) -> &ScriptRegistry {
        &self.script
    }

    /// Get a mutable reference to the underlying script registry.
    pub fn script_mut(&mut self) -> &mut ScriptRegistry {
        &mut self.script
    }

    // =========================================================================
    // Namespace Management
    // =========================================================================

    /// Get the current namespace path.
    pub fn namespace_path(&self) -> &[String] {
        &self.namespace_path
    }

    /// Enter a namespace (push onto the path).
    pub fn enter_namespace(&mut self, name: &str) {
        self.namespace_path.push(name.to_string());
    }

    /// Exit the current namespace (pop from the path).
    pub fn exit_namespace(&mut self) {
        self.namespace_path.pop();
    }

    /// Add a namespace import for name resolution.
    pub fn add_import(&mut self, namespace: &str) {
        if !self.imported_namespaces.contains(&namespace.to_string()) {
            self.imported_namespaces.push(namespace.to_string());
        }
    }

    /// Clear all namespace imports.
    pub fn clear_imports(&mut self) {
        self.imported_namespaces.clear();
    }

    /// Build a qualified name from the current namespace.
    pub fn qualified_name(&self, name: &str) -> String {
        if self.namespace_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace_path.join("::"), name)
        }
    }

    // =========================================================================
    // Name Resolution
    // =========================================================================

    /// Resolve a type name to a TypeHash using namespace rules.
    ///
    /// Resolution order:
    /// 1. Check if it's a primitive type name
    /// 2. Try fully qualified (if name contains ::)
    /// 3. Try current namespace + name
    /// 4. Try each imported namespace + name
    /// 5. Try global (just the name)
    pub fn resolve_type(&self, name: &str) -> Result<TypeHash, ResolutionError> {
        // 1. Check primitives first
        if let Some(hash) = self.primitive_hash_from_name(name) {
            return Ok(hash);
        }

        // 2. If already qualified, try direct lookup
        if name.contains("::") {
            if let Some(hash) = self.lookup_type(name) {
                return Ok(hash);
            }
            return Err(ResolutionError::UnknownType(name.to_string()));
        }

        // 3. Try current namespace
        if !self.namespace_path.is_empty() {
            let qualified = self.qualified_name(name);
            if let Some(hash) = self.lookup_type(&qualified) {
                return Ok(hash);
            }
        }

        // 4. Try imported namespaces
        for ns in &self.imported_namespaces {
            let qualified = format!("{}::{}", ns, name);
            if let Some(hash) = self.lookup_type(&qualified) {
                return Ok(hash);
            }
        }

        // 5. Try global
        if let Some(hash) = self.lookup_type(name) {
            return Ok(hash);
        }

        Err(ResolutionError::UnknownType(name.to_string()))
    }

    /// Get the primitive TypeHash for a type name, if it's a primitive.
    fn primitive_hash_from_name(&self, name: &str) -> Option<TypeHash> {
        match name {
            "void" => Some(primitives::VOID),
            "bool" => Some(primitives::BOOL),
            "int8" => Some(primitives::INT8),
            "int16" => Some(primitives::INT16),
            "int" | "int32" => Some(primitives::INT32),
            "int64" => Some(primitives::INT64),
            "uint8" => Some(primitives::UINT8),
            "uint16" => Some(primitives::UINT16),
            "uint" | "uint32" => Some(primitives::UINT32),
            "uint64" => Some(primitives::UINT64),
            "float" => Some(primitives::FLOAT),
            "double" => Some(primitives::DOUBLE),
            _ => None,
        }
    }

    // =========================================================================
    // Type Lookups (Unified)
    // =========================================================================

    /// Look up a type by name.
    ///
    /// This uses the unified name map which includes both FFI and script types.
    pub fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.type_by_name.get(name).copied()
    }

    /// Get a type definition by TypeHash.
    ///
    /// Tries FFI first, then Script. Returns None if not found.
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.ffi
            .get_type(hash)
            .or_else(|| self.script.get_type(hash))
    }

    /// Check if a type exists by TypeHash.
    pub fn has_type(&self, hash: TypeHash) -> bool {
        self.get_type(hash).is_some()
    }

    /// Get the total count of registered types (FFI + Script).
    pub fn type_count(&self) -> usize {
        self.ffi.type_count() + self.script.type_count()
    }

    // =========================================================================
    // Type Registration (delegates to ScriptRegistry)
    // =========================================================================

    /// Register a new script type and return its TypeHash.
    ///
    /// The type is added to both the ScriptRegistry and the unified name map.
    pub fn register_type(&mut self, typedef: TypeDef) -> TypeHash {
        let qualified_name = typedef.qualified_name().to_string();
        let type_hash = self.script.register_type(typedef);
        self.type_by_name.insert(qualified_name, type_hash);
        type_hash
    }

    /// Register a type with an additional name alias.
    pub fn register_type_with_alias(&mut self, typedef: TypeDef, alias: &str) -> TypeHash {
        let type_hash = self.register_type(typedef);
        self.type_by_name.insert(alias.to_string(), type_hash);
        type_hash
    }

    /// Register a type alias (typedef).
    pub fn register_type_alias(&mut self, alias_name: &str, target_type: TypeHash) {
        self.type_by_name.insert(alias_name.to_string(), target_type);
    }

    // =========================================================================
    // Function Lookups (Unified)
    // =========================================================================

    /// Look up all functions with the given name (for overload resolution).
    ///
    /// Uses the unified name map.
    pub fn lookup_functions(&self, name: &str) -> &[TypeHash] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get a function definition by func_hash.
    ///
    /// Tries FFI first, then Script. Returns None if not found.
    ///
    /// **Note**: This returns a unified `&FunctionDef` - no enum dispatch needed
    /// because both registries now use the same `FunctionDef` type from angelscript-core.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionDef> {
        self.ffi
            .get_function(hash)
            .or_else(|| self.script.get_function(hash))
    }

    /// Check if a function exists by func_hash.
    pub fn has_function(&self, hash: TypeHash) -> bool {
        self.get_function(hash).is_some()
    }

    /// Get the total count of registered functions (FFI + Script).
    pub fn function_count(&self) -> usize {
        self.ffi.function_count() + self.script.function_count()
    }

    // =========================================================================
    // Function Registration (delegates to ScriptRegistry)
    // =========================================================================

    /// Register a script function and return its func_hash.
    ///
    /// The function is added to both the ScriptRegistry and the unified name map.
    pub fn register_function(&mut self, func: FunctionDef) -> TypeHash {
        let func_hash = func.func_hash;
        let qualified_name = func.qualified_name();

        self.script.register_function(func);

        // Add to unified name map
        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_hash);

        func_hash
    }

    // =========================================================================
    // Behavior Lookups (Unified)
    // =========================================================================

    /// Get the behaviors for a type, if any are registered.
    pub fn get_behaviors(&self, type_hash: TypeHash) -> Option<&TypeBehaviors> {
        self.ffi
            .get_behaviors(type_hash)
            .or_else(|| self.script.get_behaviors(type_hash))
    }

    /// Find all constructors for a given type (value types).
    pub fn find_constructors(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        // Try FFI first
        let ffi_result = self.ffi.find_constructors(type_hash);
        if !ffi_result.is_empty() {
            ffi_result
        } else {
            self.script.find_constructors(type_hash).to_vec()
        }
    }

    /// Find all factories for a given type (reference types).
    pub fn find_factories(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        // Try FFI first
        let ffi_result = self.ffi.find_factories(type_hash);
        if !ffi_result.is_empty() {
            ffi_result
        } else {
            self.script.find_factories(type_hash).to_vec()
        }
    }

    /// Find a constructor for a type with specific argument types.
    pub fn find_constructor(&self, type_hash: TypeHash, arg_types: &[DataType]) -> Option<TypeHash> {
        self.ffi
            .find_constructor(type_hash, arg_types)
            .or_else(|| self.script.find_constructor(type_hash, arg_types))
    }

    /// Find the copy constructor for a type.
    pub fn find_copy_constructor(&self, type_hash: TypeHash) -> Option<TypeHash> {
        self.ffi
            .find_copy_constructor(type_hash)
            .or_else(|| self.script.find_copy_constructor(type_hash))
    }

    // =========================================================================
    // Method Lookups (Unified)
    // =========================================================================

    /// Get all methods for a given type.
    pub fn get_methods(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        // Try FFI first
        let ffi_result = self.ffi.get_methods(type_hash);
        if !ffi_result.is_empty() {
            ffi_result
        } else {
            self.script.get_methods(type_hash)
        }
    }

    /// Find a method by name on a type (first match).
    pub fn find_method(&self, type_hash: TypeHash, name: &str) -> Option<TypeHash> {
        self.ffi
            .find_method(type_hash, name)
            .or_else(|| self.script.find_method(type_hash, name))
    }

    /// Find all methods with the given name on a type (for overload resolution).
    pub fn find_methods_by_name(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        // Try FFI first
        let ffi_result = self.ffi.find_methods_by_name(type_hash, name);
        if !ffi_result.is_empty() {
            ffi_result
        } else {
            self.script.find_methods_by_name(type_hash, name)
        }
    }

    // =========================================================================
    // Operator Lookups (Unified)
    // =========================================================================

    /// Find an operator method on a type.
    pub fn find_operator_method(
        &self,
        type_hash: TypeHash,
        operator: OperatorBehavior,
    ) -> Option<TypeHash> {
        self.ffi
            .find_operator_method(type_hash, operator)
            .or_else(|| self.script.find_operator_method(type_hash, operator))
    }

    /// Find all overloads of an operator method for a type.
    pub fn find_operator_methods(
        &self,
        type_hash: TypeHash,
        operator: OperatorBehavior,
    ) -> Vec<TypeHash> {
        // Try FFI first
        let ffi_result = self.ffi.find_operator_methods(type_hash, operator);
        if !ffi_result.is_empty() {
            ffi_result.to_vec()
        } else {
            self.script.find_operator_methods(type_hash, operator).to_vec()
        }
    }

    // =========================================================================
    // Property Lookups (Unified)
    // =========================================================================

    /// Find a property by name on a type.
    ///
    /// Note: Returns owned value because FFI registry returns owned PropertyAccessors.
    /// Script properties are cloned to maintain a consistent API.
    pub fn find_property(&self, type_hash: TypeHash, name: &str) -> Option<PropertyAccessors> {
        self.ffi
            .find_property(type_hash, name)
            .or_else(|| self.script.find_property(type_hash, name).cloned())
    }

    // =========================================================================
    // Inheritance Support (Unified)
    // =========================================================================

    /// Get the base class of a type (if any).
    pub fn get_base_class(&self, type_hash: TypeHash) -> Option<TypeHash> {
        self.ffi
            .get_base_class(type_hash)
            .or_else(|| self.script.get_base_class(type_hash))
    }

    /// Check if `derived` is a subclass of `base`.
    pub fn is_subclass_of(&self, derived: TypeHash, base: TypeHash) -> bool {
        if derived == base {
            return true;
        }

        let mut current = self.get_base_class(derived);
        while let Some(parent) = current {
            if parent == base {
                return true;
            }
            current = self.get_base_class(parent);
        }

        false
    }

    /// Get all interfaces implemented by a class.
    pub fn get_interfaces(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        // Try FFI first
        let ffi_result = self.ffi.get_all_interfaces(type_hash);
        if !ffi_result.is_empty() {
            ffi_result
        } else {
            self.script.get_interfaces(type_hash).to_vec()
        }
    }

    // =========================================================================
    // Enum Support (Unified)
    // =========================================================================

    /// Look up an enum value by enum type hash and value name.
    pub fn lookup_enum_value(&self, type_hash: TypeHash, value_name: &str) -> Option<i64> {
        self.ffi
            .lookup_enum_value(type_hash, value_name)
            .or_else(|| self.script.lookup_enum_value(type_hash, value_name))
    }

    // =========================================================================
    // Funcdef Support (Unified)
    // =========================================================================

    /// Get the signature of a funcdef type.
    pub fn get_funcdef_signature(&self, type_hash: TypeHash) -> Option<(&[DataType], &DataType)> {
        self.ffi
            .get_funcdef_signature(type_hash)
            .or_else(|| self.script.get_funcdef_signature(type_hash))
    }

    // =========================================================================
    // Template Support (Unified)
    // =========================================================================

    /// Check if a type is a template (has template parameters).
    pub fn is_template(&self, type_hash: TypeHash) -> bool {
        self.ffi.is_template(type_hash) || self.script.is_template(type_hash)
    }

    /// Check if a type is a template instance.
    pub fn is_template_instance(&self, type_hash: TypeHash) -> bool {
        self.script.is_template_instance(type_hash)
    }
}

impl Default for CompilationContext {
    fn default() -> Self {
        Self::new(Arc::new(
            angelscript_ffi::FfiRegistryBuilder::new().build().unwrap(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_ffi::FfiRegistryBuilder;
    use crate::types::{Param, FunctionTraits, Visibility, TypeKind};

    fn create_test_context() -> CompilationContext {
        let ffi = Arc::new(FfiRegistryBuilder::new().build().unwrap());
        CompilationContext::new(ffi)
    }

    // =========================================================================
    // Basic Construction Tests
    // =========================================================================

    #[test]
    fn new_context_has_primitives() {
        let ctx = create_test_context();

        // Primitives should be accessible via unified lookup
        assert!(ctx.lookup_type("void").is_some());
        assert!(ctx.lookup_type("bool").is_some());
        assert!(ctx.lookup_type("int").is_some());
        assert!(ctx.lookup_type("int8").is_some());
        assert!(ctx.lookup_type("int16").is_some());
        assert!(ctx.lookup_type("int64").is_some());
        assert!(ctx.lookup_type("float").is_some());
        assert!(ctx.lookup_type("double").is_some());

        // Check TypeHashes match constants
        assert_eq!(ctx.lookup_type("void"), Some(primitives::VOID));
        assert_eq!(ctx.lookup_type("int"), Some(primitives::INT32));
        assert_eq!(ctx.lookup_type("bool"), Some(primitives::BOOL));
    }

    #[test]
    fn context_default() {
        let ctx = CompilationContext::default();
        assert!(ctx.lookup_type("int").is_some());
    }

    #[test]
    fn context_debug() {
        let ctx = create_test_context();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("CompilationContext"));
    }

    // =========================================================================
    // Namespace Tests
    // =========================================================================

    #[test]
    fn namespace_management() {
        let mut ctx = create_test_context();

        assert!(ctx.namespace_path().is_empty());
        assert_eq!(ctx.qualified_name("Player"), "Player");

        ctx.enter_namespace("Game");
        assert_eq!(ctx.namespace_path(), &["Game"]);
        assert_eq!(ctx.qualified_name("Player"), "Game::Player");

        ctx.enter_namespace("Entities");
        assert_eq!(ctx.namespace_path(), &["Game", "Entities"]);
        assert_eq!(ctx.qualified_name("Player"), "Game::Entities::Player");

        ctx.exit_namespace();
        assert_eq!(ctx.namespace_path(), &["Game"]);

        ctx.exit_namespace();
        assert!(ctx.namespace_path().is_empty());
    }

    #[test]
    fn import_management() {
        let mut ctx = create_test_context();

        ctx.add_import("Game::Entities");
        ctx.add_import("Game::Utils");
        ctx.add_import("Game::Entities"); // Duplicate - should not add

        assert_eq!(ctx.imported_namespaces.len(), 2);

        ctx.clear_imports();
        assert!(ctx.imported_namespaces.is_empty());
    }

    // =========================================================================
    // Type Resolution Tests
    // =========================================================================

    #[test]
    fn resolve_primitive_types() {
        let ctx = create_test_context();

        assert_eq!(ctx.resolve_type("int").unwrap(), primitives::INT32);
        assert_eq!(ctx.resolve_type("float").unwrap(), primitives::FLOAT);
        assert_eq!(ctx.resolve_type("void").unwrap(), primitives::VOID);
    }

    #[test]
    fn resolve_unknown_type_error() {
        let ctx = create_test_context();

        let result = ctx.resolve_type("UnknownType");
        assert!(matches!(result, Err(ResolutionError::UnknownType(_))));
    }

    #[test]
    fn resolve_type_in_namespace() {
        let mut ctx = create_test_context();

        // Register a type in Game namespace
        let typedef = make_class("Game::Player");
        ctx.register_type(typedef);

        // Enter Game namespace
        ctx.enter_namespace("Game");

        // Should resolve unqualified name from current namespace
        assert!(ctx.resolve_type("Player").is_ok());

        // Should also resolve fully qualified
        assert!(ctx.resolve_type("Game::Player").is_ok());
    }

    #[test]
    fn resolve_type_from_import() {
        let mut ctx = create_test_context();

        // Register a type in Game::Entities namespace
        let typedef = make_class("Game::Entities::Player");
        ctx.register_type(typedef);

        // Add import
        ctx.add_import("Game::Entities");

        // Should resolve via import
        assert!(ctx.resolve_type("Player").is_ok());
    }

    // =========================================================================
    // Type Registration Tests
    // =========================================================================

    #[test]
    fn register_script_type() {
        let mut ctx = create_test_context();

        let typedef = make_class("Player");
        let type_hash = ctx.register_type(typedef);

        // Should be findable via unified lookup
        assert_eq!(ctx.lookup_type("Player"), Some(type_hash));

        // Should be retrievable
        let retrieved = ctx.get_type(type_hash).unwrap();
        assert!(retrieved.is_class());
    }

    #[test]
    fn register_type_with_alias() {
        let mut ctx = create_test_context();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
            type_hash: TypeHash::from_name("Game::Player"),
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };

        let type_hash = ctx.register_type_with_alias(typedef, "Player");

        // Can lookup by qualified name
        assert_eq!(ctx.lookup_type("Game::Player"), Some(type_hash));
        // Can also lookup by alias
        assert_eq!(ctx.lookup_type("Player"), Some(type_hash));
    }

    #[test]
    fn register_type_alias() {
        let mut ctx = create_test_context();

        ctx.register_type_alias("integer", primitives::INT32);

        assert_eq!(ctx.lookup_type("integer"), Some(primitives::INT32));
        assert_eq!(ctx.lookup_type("int"), Some(primitives::INT32));
    }

    // =========================================================================
    // Function Tests
    // =========================================================================

    #[test]
    fn register_and_get_function() {
        let mut ctx = create_test_context();

        let func = make_function("add", vec![
            Param::new("a", DataType::simple(primitives::INT32)),
            Param::new("b", DataType::simple(primitives::INT32)),
        ], DataType::simple(primitives::INT32));

        let func_hash = func.func_hash;
        ctx.register_function(func);

        // Should be findable via lookup
        let overloads = ctx.lookup_functions("add");
        assert_eq!(overloads.len(), 1);
        assert_eq!(overloads[0], func_hash);

        // Should be retrievable - returns &FunctionDef directly (no enum!)
        let retrieved = ctx.get_function(func_hash).unwrap();
        assert_eq!(retrieved.name, "add");
        assert_eq!(retrieved.params.len(), 2);
    }

    #[test]
    fn function_overloads() {
        let mut ctx = create_test_context();

        // Register two overloads of "print"
        let print_int = make_function(
            "print",
            vec![Param::new("val", DataType::simple(primitives::INT32))],
            DataType::void(),
        );
        let print_float = make_function(
            "print",
            vec![Param::new("val", DataType::simple(primitives::FLOAT))],
            DataType::void(),
        );

        let hash1 = ctx.register_function(print_int);
        let hash2 = ctx.register_function(print_float);

        let overloads = ctx.lookup_functions("print");
        assert_eq!(overloads.len(), 2);
        assert!(overloads.contains(&hash1));
        assert!(overloads.contains(&hash2));
    }

    // =========================================================================
    // Unified Lookup Tests
    // =========================================================================

    #[test]
    fn get_type_routes_correctly() {
        let ctx = create_test_context();

        // FFI type (primitive)
        let void_type = ctx.get_type(primitives::VOID);
        assert!(void_type.is_some());
        assert!(void_type.unwrap().is_primitive());
    }

    #[test]
    fn get_function_returns_function_def_not_enum() {
        let mut ctx = create_test_context();

        let func = make_function("test", vec![], DataType::void());
        let hash = ctx.register_function(func);

        // This should return Option<&FunctionDef>, not FunctionRef enum
        let retrieved: Option<&FunctionDef> = ctx.get_function(hash);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    // =========================================================================
    // Inheritance Tests
    // =========================================================================

    #[test]
    fn is_subclass_of() {
        let mut ctx = create_test_context();

        // Register base and derived classes
        let base_hash = TypeHash::from_name("Base");
        let base = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: base_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        ctx.register_type(base);

        let derived_hash = TypeHash::from_name("Derived");
        let derived = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: derived_hash,
            fields: vec![],
            methods: vec![],
            base_class: Some(base_hash),
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        ctx.register_type(derived);

        assert!(ctx.is_subclass_of(derived_hash, base_hash));
        assert!(ctx.is_subclass_of(base_hash, base_hash)); // Same class
        assert!(!ctx.is_subclass_of(base_hash, derived_hash)); // Not the other way
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn make_class(qualified_name: &str) -> TypeDef {
        let name = qualified_name.split("::").last().unwrap_or(qualified_name);
        let type_hash = TypeHash::from_name(qualified_name);
        TypeDef::Class {
            name: name.to_string(),
            qualified_name: qualified_name.to_string(),
            type_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        }
    }

    fn make_function(name: &str, params: Vec<Param>, return_type: DataType) -> FunctionDef {
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        FunctionDef {
            func_hash: TypeHash::from_function(name, &param_hashes),
            name: name.to_string(),
            namespace: vec![],
            params,
            return_type,
            object_type: None,
            traits: FunctionTraits::default(),
            is_native: false,
            visibility: Visibility::Public,
        }
    }
}
