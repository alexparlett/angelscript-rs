# Phase 6: Completion Pass Rewrite

## Overview

Rewrite the Completion pass to:
1. Take `RegistrationResult` as input
2. **Resolve using directives** into namespace tree edges (FIRST)
3. Build a name index for resolution
4. Transform unresolved entries into resolved entries
5. Populate the registry (namespace tree)
6. Handle inheritance, vtables, and hash indexes

**Files:**
- `crates/angelscript-compiler/src/passes/completion.rs` (rewrite)

---

## CompletionPass Structure

```rust
// crates/angelscript-compiler/src/passes/completion.rs

use angelscript_core::{
    ClassEntry, CompilationError, DataType, EnumEntry, FuncdefEntry, FunctionDef,
    FunctionEntry, GlobalPropertyEntry, InterfaceEntry, MethodSignature, Param,
    PropertyEntry, QualifiedName, Span, TypeEntry, TypeHash, TypeKind, TypeSource,
    UnresolvedClass, UnresolvedType, Visibility,
};
use angelscript_registry::{SymbolRegistry, ResolutionContext, ResolutionResult};
use rustc_hash::FxHashMap;

use crate::passes::RegistrationResult;

/// Output of Pass 2 (Completion).
#[derive(Debug, Default)]
pub struct CompletionResult {
    /// Number of types resolved and registered.
    pub types_registered: usize,
    /// Number of functions resolved and registered.
    pub functions_registered: usize,
    /// Number of globals resolved and registered.
    pub globals_registered: usize,
    /// Number of vtables built.
    pub vtables_built: usize,
    /// Errors encountered during completion.
    pub errors: Vec<CompilationError>,
}

impl CompletionResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Pass 2: Complete type information and populate registry.
///
/// Takes `RegistrationResult` from Pass 1, resolves all types,
/// transforms unresolved entries to resolved entries, and
/// populates the registry.
pub struct CompletionPass<'reg, 'global> {
    /// Unit registry to populate.
    registry: &'reg mut SymbolRegistry,
    /// Global registry for FFI type lookup.
    global_registry: &'global SymbolRegistry,
    /// Temporary name index for resolution (built from RegistrationResult).
    name_index: FxHashMap<QualifiedName, UnresolvedTypeKind>,
    /// Result being built.
    result: CompletionResult,
}

/// Kind of unresolved type (for name index).
#[derive(Debug, Clone, Copy)]
enum UnresolvedTypeKind {
    Class,
    Mixin,
    Interface,
    Funcdef,
    Enum,
}
```

---

## Main Entry Point

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    pub fn new(
        registry: &'reg mut SymbolRegistry,
        global_registry: &'global SymbolRegistry,
    ) -> Self {
        Self {
            registry,
            global_registry,
            name_index: FxHashMap::default(),
            result: CompletionResult::default(),
        }
    }

    /// Run the completion pass.
    pub fn run(mut self, input: RegistrationResult) -> CompletionResult {
        // Phase 0: Resolve using directives FIRST
        // Creates namespace tree edges for using namespace resolution.
        // Must happen before type resolution so lookups can traverse using edges.
        self.resolve_using_directives(&input);

        // Phase 1: Build name index from unresolved entries
        self.build_name_index(&input);

        // Phase 2: Register all types (creates entries in registry namespace tree)
        self.register_types(&input);

        // Phase 3: Resolve and register functions
        self.register_functions(&input);

        // Phase 4: Resolve and register globals
        self.register_globals(&input);

        // Phase 5: Resolve inheritance relationships
        self.resolve_inheritance(&input);

        // Phase 6: Complete class members (copy from base, apply mixins)
        self.complete_class_members();

        // Phase 7: Build vtables and itables
        self.build_vtables();

        // Phase 8: Build hash indexes for bytecode
        self.registry.build_hash_indexes();

        self.result
    }
}
```

---

## Phase 0: Resolve Using Directives

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    /// Resolve using directives to graph edges.
    ///
    /// The namespace tree was already built during the Registration pass.
    /// This MUST happen before type resolution so that the namespace tree
    /// can traverse `Uses` edges when looking up types.
    fn resolve_using_directives(&mut self, input: &RegistrationResult) {
        for directive in &input.using_directives {
            if let Err(e) = self.resolve_single_using_directive(directive) {
                self.result.errors.push(e);
            }
        }
    }

    fn resolve_single_using_directive(
        &mut self,
        directive: &UnresolvedUsingDirective,
    ) -> Result<(), CompilationError> {
        let tree = self.registry.tree_mut();

        // Get source namespace (must exist - it was created during registration)
        let source_node = tree.get_path(&directive.source_namespace)
            .ok_or_else(|| CompilationError::InternalError {
                message: format!("Source namespace {} not found", directive.source_namespace.join("::")),
                span: directive.span,
            })?;

        // Get target namespace (must exist, or it's an error)
        let target_node = tree.get_path(&directive.target_namespace)
            .ok_or_else(|| CompilationError::UnknownNamespace {
                name: directive.target_namespace.join("::"),
                span: directive.span,
            })?;

        // Add the Uses edge (non-transitive)
        tree.add_using_directive(source_node, target_node);

        Ok(())
    }
}
```

---

## Phase 1: Build Name Index

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    /// Build a name index from unresolved entries for type resolution.
    fn build_name_index(&mut self, input: &RegistrationResult) {
        for class in &input.classes {
            self.name_index.insert(class.name.clone(), UnresolvedTypeKind::Class);
        }
        for mixin in &input.mixins {
            self.name_index.insert(mixin.class.name.clone(), UnresolvedTypeKind::Mixin);
        }
        for iface in &input.interfaces {
            self.name_index.insert(iface.name.clone(), UnresolvedTypeKind::Interface);
        }
        for funcdef in &input.funcdefs {
            self.name_index.insert(funcdef.name.clone(), UnresolvedTypeKind::Funcdef);
        }
        for e in &input.enums {
            self.name_index.insert(e.name.clone(), UnresolvedTypeKind::Enum);
        }
    }

    /// Check if a name exists in the combined registries.
    fn type_exists(&self, name: &QualifiedName) -> bool {
        self.name_index.contains_key(name)
            || self.registry.contains_type_name(name)
            || self.global_registry.contains_type_name(name)
    }

    /// Resolve an UnresolvedType to a QualifiedName.
    ///
    /// Uses the namespace tree for resolution, which handles:
    /// 1. Current namespace and ancestors (walk up to root)
    /// 2. Using directive namespaces (via `Uses` edges, non-transitive)
    fn resolve_type_name(&self, unresolved: &UnresolvedType) -> Result<QualifiedName, CompilationError> {
        // Handle void
        if unresolved.is_void() {
            return Ok(QualifiedName::global("void"));
        }

        // Handle primitives
        if is_primitive(&unresolved.name) {
            return Ok(QualifiedName::global(&unresolved.name));
        }

        // Handle template types (e.g., "array<int>")
        if unresolved.is_template() {
            return self.resolve_template_type_name(unresolved);
        }

        // Try qualified name directly
        if unresolved.is_qualified() {
            let qn = QualifiedName::from_qualified_string(&unresolved.name);
            if self.type_exists(&qn) {
                return Ok(qn);
            }
            return Err(CompilationError::UnknownType {
                name: unresolved.name.clone(),
                span: unresolved.span,
            });
        }

        // Use namespace tree resolution which handles:
        // 1. Current namespace and walk up to root
        // 2. Using directive namespaces at current and parent scopes (non-transitive)
        //
        // Note: We use get_path (not get_or_create_path) because the namespace tree
        // was already built during Registration. If the namespace doesn't exist,
        // fall back to root.
        let ctx = ResolutionContext {
            current_namespace: self.registry.tree().get_path(&unresolved.context_namespace)
                .unwrap_or_else(|| self.registry.tree().root()),
        };

        // Use checked resolution to detect ambiguity
        match self.registry.tree().resolve_type_with_location_checked(&unresolved.name, &ctx) {
            ResolutionResult::Found((entry, ns_node)) => {
                let path = self.registry.tree().get_namespace_path(ns_node);
                return Ok(QualifiedName::new(entry.simple_name(), path));
            }
            ResolutionResult::Ambiguous(matches) => {
                let candidates: Vec<String> = matches.iter()
                    .map(|(ns, (entry, _))| {
                        let path = self.registry.tree().get_namespace_path(*ns);
                        QualifiedName::new(entry.simple_name(), path).to_string()
                    })
                    .collect();
                return Err(CompilationError::AmbiguousType {
                    name: unresolved.name.clone(),
                    candidates,
                    span: unresolved.span,
                });
            }
            ResolutionResult::NotFound => {
                // Continue to check global registry
            }
        }

        // Also check global registry for FFI types
        if let Some(qn) = self.global_registry.resolve_type_name(
            &unresolved.name,
            &unresolved.context_namespace,
        ) {
            return Ok(qn);
        }

        Err(CompilationError::UnknownType {
            name: unresolved.name.clone(),
            span: unresolved.span,
        })
    }

    /// Resolve an UnresolvedType to a DataType.
    fn resolve_type(&self, unresolved: &UnresolvedType) -> Result<DataType, CompilationError> {
        // Handle void
        if unresolved.is_void() {
            return Ok(DataType::void());
        }

        // Handle primitives
        if let Some(dt) = resolve_primitive(&unresolved.name) {
            return Ok(self.apply_modifiers(dt, unresolved));
        }

        // Handle template types
        if unresolved.is_template() {
            return self.resolve_template_type(unresolved);
        }

        // Resolve name
        let qn = self.resolve_type_name(unresolved)?;

        // Get type hash (compute from name if not yet in registry)
        let type_hash = self.get_or_compute_hash(&qn)?;

        // Build DataType with flags
        let mut dt = DataType::simple(type_hash);

        // Apply type flags based on what kind of type it is
        if let Some(kind) = self.name_index.get(&qn) {
            match kind {
                UnresolvedTypeKind::Interface => dt = dt.with_is_interface(true),
                UnresolvedTypeKind::Mixin => dt = dt.with_is_mixin(true),
                UnresolvedTypeKind::Enum => dt = dt.with_is_enum(true),
                _ => {}
            }
        }

        Ok(self.apply_modifiers(dt, unresolved))
    }

    fn apply_modifiers(&self, mut dt: DataType, unresolved: &UnresolvedType) -> DataType {
        if unresolved.is_const {
            dt = dt.with_const(true);
        }
        if unresolved.is_handle {
            dt = dt.with_handle(true);
        }
        if unresolved.is_handle_to_const {
            dt = dt.with_handle_to_const(true);
        }
        dt = dt.with_ref_modifier(unresolved.ref_modifier);
        dt
    }

    fn get_or_compute_hash(&self, name: &QualifiedName) -> Result<TypeHash, CompilationError> {
        // Check if already in registry
        if let Some(hash) = self.registry.get_type_hash(name) {
            return Ok(hash);
        }
        if let Some(hash) = self.global_registry.get_type_hash(name) {
            return Ok(hash);
        }
        // Compute from name
        Ok(name.to_type_hash())
    }
}
```

---

## Phase 2: Register Types

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    fn register_types(&mut self, input: &RegistrationResult) {
        // Register enums first (they have no dependencies)
        for e in &input.enums {
            if let Err(err) = self.register_enum(e) {
                self.result.errors.push(err);
            } else {
                self.result.types_registered += 1;
            }
        }

        // Register funcdefs (may depend on other types, but not classes)
        for fd in &input.funcdefs {
            if let Err(err) = self.register_funcdef(fd) {
                self.result.errors.push(err);
            } else {
                self.result.types_registered += 1;
            }
        }

        // Register interfaces (may depend on other interfaces)
        for iface in &input.interfaces {
            if let Err(err) = self.register_interface(iface) {
                self.result.errors.push(err);
            } else {
                self.result.types_registered += 1;
            }
        }

        // Register classes (may depend on anything)
        for class in &input.classes {
            if let Err(err) = self.register_class(class) {
                self.result.errors.push(err);
            } else {
                self.result.types_registered += 1;
            }
        }

        // Register mixins
        for mixin in &input.mixins {
            if let Err(err) = self.register_mixin(mixin) {
                self.result.errors.push(err);
            } else {
                self.result.types_registered += 1;
            }
        }
    }

    fn register_class(&mut self, unresolved: &UnresolvedClass) -> Result<(), CompilationError> {
        let type_hash = unresolved.name.to_type_hash();
        let source = TypeSource::script(unresolved.unit_id, unresolved.span);

        let mut entry = ClassEntry::new_with_name(
            unresolved.name.clone(),
            type_hash,
            TypeKind::ScriptObject,
            source,
        );

        // Apply modifiers
        if unresolved.is_final {
            entry.is_final = true;
        }
        if unresolved.is_abstract {
            entry.is_abstract = true;
        }

        // Resolve and add fields
        for field in &unresolved.fields {
            match self.resolve_field(field) {
                Ok(prop) => entry.properties.push(prop),
                Err(e) => self.result.errors.push(e),
            }
        }

        // Note: Methods, inheritance, vtables handled in later phases

        self.registry.register_type_with_name(entry.into(), unresolved.name.clone())
            .map_err(|e| CompilationError::Other {
                message: e.to_string(),
                span: unresolved.span,
            })
    }

    fn register_interface(&mut self, unresolved: &UnresolvedInterface) -> Result<(), CompilationError> {
        let type_hash = unresolved.name.to_type_hash();
        let source = TypeSource::script(unresolved.unit_id, unresolved.span);

        let mut entry = InterfaceEntry::new(
            unresolved.name.simple_name(),
            unresolved.name.namespace_path().to_vec(),
            unresolved.name.to_string(),
            type_hash,
            source,
        );

        // Set qualified_name field
        entry.qualified_name = unresolved.name.clone();

        // Resolve method signatures
        for method in &unresolved.methods {
            match self.resolve_method_signature(method) {
                Ok(sig) => entry.methods.push(sig),
                Err(e) => self.result.errors.push(e),
            }
        }

        self.registry.register_type_with_name(entry.into(), unresolved.name.clone())
            .map_err(|e| CompilationError::Other {
                message: e.to_string(),
                span: unresolved.span,
            })
    }

    fn register_enum(&mut self, unresolved: &UnresolvedEnum) -> Result<(), CompilationError> {
        let type_hash = unresolved.name.to_type_hash();
        let source = TypeSource::script(unresolved.unit_id, unresolved.span);

        let mut entry = EnumEntry::new(
            unresolved.name.simple_name(),
            unresolved.name.namespace_path().to_vec(),
            unresolved.name.to_string(),
            type_hash,
            source,
        );

        // Add enum values
        let mut next_value: i64 = 0;
        for value in &unresolved.values {
            let actual_value = value.explicit_value.unwrap_or(next_value);
            entry.add_value(&value.name, actual_value);
            next_value = actual_value + 1;
        }

        self.registry.register_type_with_name(entry.into(), unresolved.name.clone())
            .map_err(|e| CompilationError::Other {
                message: e.to_string(),
                span: unresolved.span,
            })
    }

    fn register_funcdef(&mut self, unresolved: &UnresolvedFuncdef) -> Result<(), CompilationError> {
        let type_hash = unresolved.name.to_type_hash();
        let source = TypeSource::script(unresolved.unit_id, unresolved.span);

        // Resolve params
        let params: Vec<DataType> = unresolved.params.iter()
            .map(|p| self.resolve_type(&p.param_type))
            .collect::<Result<Vec<_>, _>>()?;

        let return_type = self.resolve_type(&unresolved.return_type)?;

        let entry = FuncdefEntry::new(
            unresolved.name.simple_name(),
            unresolved.name.namespace_path().to_vec(),
            unresolved.name.to_string(),
            type_hash,
            source,
            params,
            return_type,
        );

        self.registry.register_type_with_name(entry.into(), unresolved.name.clone())
            .map_err(|e| CompilationError::Other {
                message: e.to_string(),
                span: unresolved.span,
            })
    }
}
```

---

## Phase 3: Register Functions

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    fn register_functions(&mut self, input: &RegistrationResult) {
        // Global functions
        for func in &input.functions {
            if let Err(e) = self.register_function(func) {
                self.result.errors.push(e);
            } else {
                self.result.functions_registered += 1;
            }
        }

        // Class methods
        for class in &input.classes {
            for method in &class.methods {
                if let Err(e) = self.register_method(method, &class.name) {
                    self.result.errors.push(e);
                } else {
                    self.result.functions_registered += 1;
                }
            }
        }
    }

    fn register_function(&mut self, unresolved: &UnresolvedFunction) -> Result<(), CompilationError> {
        // Resolve params
        let params: Vec<Param> = unresolved.signature.params.iter()
            .map(|p| {
                let dt = self.resolve_type(&p.param_type)?;
                Ok(Param::new(&p.name, dt).with_has_default(p.has_default))
            })
            .collect::<Result<Vec<_>, CompilationError>>()?;

        let return_type = self.resolve_type(&unresolved.signature.return_type)?;

        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        let func_hash = TypeHash::from_function(&unresolved.name.to_string(), &param_hashes);

        let func_def = FunctionDef::new(
            func_hash,
            unresolved.name.simple_name(),
            unresolved.name.namespace_path().to_vec(),
            params,
            return_type,
            None, // No object type for global functions
            FunctionTraits::default(),
            false,
            unresolved.visibility,
        );

        let source = FunctionSource::script(unresolved.span);
        let entry = FunctionEntry::script(func_def, unresolved.unit_id, source);

        self.registry.register_function_with_name(entry, unresolved.name.clone())
            .map_err(|e| CompilationError::Other {
                message: e.to_string(),
                span: unresolved.span,
            })
    }

    fn register_method(
        &mut self,
        method: &UnresolvedMethod,
        class_name: &QualifiedName,
    ) -> Result<(), CompilationError> {
        let class_hash = class_name.to_type_hash();

        // Resolve params
        let params: Vec<Param> = method.signature.params.iter()
            .map(|p| {
                let dt = self.resolve_type(&p.param_type)?;
                Ok(Param::new(&p.name, dt).with_has_default(p.has_default))
            })
            .collect::<Result<Vec<_>, CompilationError>>()?;

        let return_type = self.resolve_type(&method.signature.return_type)?;

        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        let func_hash = TypeHash::from_method(class_hash, &method.name, &param_hashes);

        let mut traits = FunctionTraits {
            is_virtual: method.is_virtual,
            is_override: method.is_override,
            is_final: method.is_final,
            is_abstract: method.is_abstract,
            is_const: method.is_const,
            ..Default::default()
        };

        let func_def = FunctionDef::new(
            func_hash,
            &method.name,
            Vec::new(),
            params,
            return_type,
            Some(class_hash),
            traits,
            false,
            method.visibility,
        );

        let source = FunctionSource::script(method.span);
        let entry = FunctionEntry::script(func_def, unresolved.unit_id, source);

        let method_name = QualifiedName::new(&method.name, class_name.namespace_path().to_vec());
        self.registry.register_function_with_name(entry, method_name)
            .map_err(|e| CompilationError::Other {
                message: e.to_string(),
                span: method.span,
            })?;

        // Add method to class
        if let Some(class) = self.registry.get_class_mut(class_hash) {
            class.add_method(&method.name, func_hash);
        }

        Ok(())
    }
}
```

---

## Phase 5-7: Inheritance and VTables

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    fn resolve_inheritance(&mut self, input: &RegistrationResult) {
        for class in &input.classes {
            if let Err(e) = self.resolve_class_inheritance(class) {
                self.result.errors.push(e);
            }
        }
    }

    fn resolve_class_inheritance(&mut self, unresolved: &UnresolvedClass) -> Result<(), CompilationError> {
        let class_hash = unresolved.name.to_type_hash();

        let mut base_class: Option<TypeHash> = None;
        let mut mixins: Vec<TypeHash> = Vec::new();
        let mut interfaces: Vec<TypeHash> = Vec::new();

        for inherit in &unresolved.inheritance {
            let resolved_name = self.resolve_type_name(&inherit.type_ref)?;
            let resolved_hash = resolved_name.to_type_hash();

            // Determine what kind of type it is
            if let Some(kind) = self.name_index.get(&resolved_name) {
                match kind {
                    UnresolvedTypeKind::Interface => {
                        interfaces.push(resolved_hash);
                    }
                    UnresolvedTypeKind::Mixin => {
                        mixins.push(resolved_hash);
                    }
                    UnresolvedTypeKind::Class => {
                        if base_class.is_some() {
                            return Err(CompilationError::MultipleInheritance {
                                class_name: unresolved.name.to_string(),
                                span: inherit.span,
                            });
                        }
                        base_class = Some(resolved_hash);
                    }
                    _ => {
                        return Err(CompilationError::InvalidInheritance {
                            message: format!("{} cannot be inherited", resolved_name),
                            span: inherit.span,
                        });
                    }
                }
            } else if self.global_registry.contains_type_name(&resolved_name) {
                // FFI type - check what kind
                if let Some(entry) = self.global_registry.get_type_by_name(&resolved_name) {
                    if entry.as_interface().is_some() {
                        interfaces.push(resolved_hash);
                    } else if entry.as_class().is_some() {
                        if base_class.is_some() {
                            return Err(CompilationError::MultipleInheritance {
                                class_name: unresolved.name.to_string(),
                                span: inherit.span,
                            });
                        }
                        base_class = Some(resolved_hash);
                    }
                }
            }
        }

        // Update class entry
        if let Some(class) = self.registry.get_class_mut(class_hash) {
            class.base_class = base_class;
            class.mixins = mixins;
            class.interfaces = interfaces;
        }

        Ok(())
    }

    fn complete_class_members(&mut self) {
        // Get all classes in dependency order
        let class_hashes: Vec<TypeHash> = self.registry
            .classes()
            .map(|c| c.type_hash)
            .collect();

        // Topological sort by inheritance
        let ordered = match self.topological_sort(&class_hashes) {
            Ok(o) => o,
            Err(e) => {
                self.result.errors.push(e);
                return;
            }
        };

        // Process each class
        for class_hash in ordered {
            self.complete_single_class(class_hash);
        }
    }

    fn build_vtables(&mut self) {
        let class_hashes: Vec<TypeHash> = self.registry
            .classes()
            .filter(|c| !c.is_mixin)
            .map(|c| c.type_hash)
            .collect();

        // Build interface itables first
        self.build_interface_itables();

        // Build class vtables
        for class_hash in class_hashes {
            if let Err(e) = self.build_class_vtable(class_hash) {
                self.result.errors.push(e);
            } else {
                self.result.vtables_built += 1;
            }
        }
    }
}
```

---

## Helper Functions

```rust
fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "void" | "bool" | "int8" | "int16" | "int" | "int32" | "int64"
            | "uint8" | "uint16" | "uint" | "uint32" | "uint64"
            | "float" | "double"
    )
}

fn resolve_primitive(name: &str) -> Option<DataType> {
    use angelscript_core::primitives;
    match name {
        "void" => Some(DataType::void()),
        "bool" => Some(DataType::simple(primitives::BOOL)),
        "int8" => Some(DataType::simple(primitives::INT8)),
        "int16" => Some(DataType::simple(primitives::INT16)),
        "int" | "int32" => Some(DataType::simple(primitives::INT32)),
        "int64" => Some(DataType::simple(primitives::INT64)),
        "uint8" => Some(DataType::simple(primitives::UINT8)),
        "uint16" => Some(DataType::simple(primitives::UINT16)),
        "uint" | "uint32" => Some(DataType::simple(primitives::UINT32)),
        "uint64" => Some(DataType::simple(primitives::UINT64)),
        "float" => Some(DataType::simple(primitives::FLOAT)),
        "double" => Some(DataType::simple(primitives::DOUBLE)),
        _ => None,
    }
}
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::passes::RegistrationPass;
    use angelscript_parser::parse_script;

    fn complete(source: &str) -> (SymbolRegistry, CompletionResult) {
        let ast = parse_script(source).unwrap();
        let reg_result = RegistrationPass::new(UnitId::new(0)).run(&ast);

        let global_registry = SymbolRegistry::new();
        let mut unit_registry = SymbolRegistry::new();

        let completion_result = CompletionPass::new(&mut unit_registry, &global_registry)
            .run(reg_result);

        (unit_registry, completion_result)
    }

    #[test]
    fn complete_simple_class() {
        let (registry, result) = complete("class Player {}");

        assert!(!result.has_errors());
        assert_eq!(result.types_registered, 1);

        let name = QualifiedName::global("Player");
        assert!(registry.get_type_by_name(&name).is_some());
    }

    #[test]
    fn complete_forward_reference() {
        let (registry, result) = complete(r#"
            interface IDamageable {
                void attack(Player@ p);
            }
            class Player : IDamageable {
                void attack(Player@ p) {}
            }
        "#);

        assert!(!result.has_errors(), "Errors: {:?}", result.errors);
        assert_eq!(result.types_registered, 2);

        // Both types resolved
        assert!(registry.get_type_by_name(&QualifiedName::global("IDamageable")).is_some());
        assert!(registry.get_type_by_name(&QualifiedName::global("Player")).is_some());
    }

    #[test]
    fn complete_circular_reference() {
        let (registry, result) = complete(r#"
            class Foo { void use(Bar@ b) {} }
            class Bar { void use(Foo@ f) {} }
        "#);

        assert!(!result.has_errors());
        assert!(registry.get_type_by_name(&QualifiedName::global("Foo")).is_some());
        assert!(registry.get_type_by_name(&QualifiedName::global("Bar")).is_some());
    }

    #[test]
    fn complete_namespace_resolution() {
        let (registry, result) = complete(r#"
            namespace Game {
                class Entity {}
                class Player {
                    Entity@ owner;
                }
            }
        "#);

        assert!(!result.has_errors());

        let player = registry.get_type_by_name(
            &QualifiedName::new("Player", vec!["Game".into()])
        ).unwrap();

        // Field type should be resolved to Game::Entity
        let class = player.as_class().unwrap();
        // Check field has correct type hash
    }

    #[test]
    fn complete_using_namespace() {
        let (registry, result) = complete(r#"
            namespace Utils {
                class Helper {}
            }
            namespace Game {
                using Utils;
                class Player {
                    Helper@ helper;  // Resolved via using directive
                }
            }
        "#);

        assert!(!result.has_errors(), "Errors: {:?}", result.errors);

        // Player should have field with Utils::Helper type
        let player = registry.get_type_by_name(
            &QualifiedName::new("Player", vec!["Game".into()])
        ).unwrap();
        let class = player.as_class().unwrap();
        // Field type hash should match Utils::Helper
    }

    #[test]
    fn using_namespace_not_transitive() {
        // A uses B, B uses C -> A should NOT see C
        let (_registry, result) = complete(r#"
            namespace C {
                class CType {}
            }
            namespace B {
                using C;
                class BType {}
            }
            namespace A {
                using B;
                class AType {
                    CType@ c;  // ERROR: CType not visible (using is non-transitive)
                }
            }
        "#);

        // Should have an error - CType not found
        assert!(result.has_errors());
    }

    #[test]
    fn unknown_using_namespace_error() {
        let (_registry, result) = complete(r#"
            namespace Game {
                using DoesNotExist;
            }
        "#);

        // Should have an error - namespace doesn't exist
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| {
            matches!(e, CompilationError::UnknownNamespace { .. })
        }));
    }

    #[test]
    fn ambiguous_type_error() {
        // A uses both B and C, which both define Helper
        let (_registry, result) = complete(r#"
            namespace B {
                class Helper {}
            }
            namespace C {
                class Helper {}
            }
            namespace A {
                using B;
                using C;
                class AType {
                    Helper@ h;  // ERROR: Ambiguous - could be B::Helper or C::Helper
                }
            }
        "#);

        // Should have an ambiguity error
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| {
            matches!(e, CompilationError::AmbiguousType { .. })
        }));
    }

    #[test]
    fn explicit_global_scope_no_using() {
        // ::Name should NOT use using directives
        let (_registry, result) = complete(r#"
            namespace Utils {
                class Helper {}
            }
            namespace Game {
                using Utils;
                class Player {
                    ::Helper@ h;  // ERROR: Helper is not in global scope
                }
            }
        "#);

        // Should have an error - Helper not in global scope
        assert!(result.has_errors());
        assert!(result.errors.iter().any(|e| {
            matches!(e, CompilationError::UnknownType { .. })
        }));
    }

    #[test]
    fn parent_scope_using_inherited() {
        // Child namespace should inherit parent's using directives
        let (registry, result) = complete(r#"
            namespace Utils {
                class Helper {}
            }
            namespace Game {
                using Utils;
                namespace Entities {
                    class Player {
                        Helper@ h;  // Should resolve via parent's using directive
                    }
                }
            }
        "#);

        assert!(!result.has_errors(), "Errors: {:?}", result.errors);

        // Player should have field with Utils::Helper type
        let player = registry.get_type_by_name(
            &QualifiedName::new("Player", vec!["Game".into(), "Entities".into()])
        ).unwrap();
        assert!(player.as_class().is_some());
    }
}
```

---

## Dependencies

- Phase 1-3: Core types and RegistrationResult (including `UnresolvedUsingDirective`)
- Phase 4: Updated registry with namespace tree (`NamespaceTree` from 04b design)
- Phase 4b: Namespace tree design with `Uses` edges for using directive resolution

---

## What's Next

Phase 7 updates the Compilation pass to use the fully-resolved registry.
