# Phase 5: Completion Pass Updates

## Overview

Update the Completion pass to:
1. **Resolve all `UnresolvedType`** references to `QualifiedName`
2. **Resolve function signatures** (params and return types)
3. **Build TypeHash indexes** after all resolution is complete
4. Continue existing inheritance, mixin, and vtable building

**Files:**
- `crates/angelscript-compiler/src/passes/completion.rs` (update)

---

## New Phase Structure

The completion pass now has additional phases:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          COMPLETION PASS                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Phase 1: Type Resolution (NEW)                                          │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │ - Resolve InheritanceRef::Unresolved → InheritanceRef::Resolved│     │
│  │ - Resolve UnresolvedSignature → MethodSignature                │     │
│  │ - Resolve FunctionDef unresolved params/return                 │     │
│  │ - Resolve FuncdefEntry unresolved params/return                │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                               │                                          │
│                               v                                          │
│  Phase 2: Inheritance Resolution (existing)                              │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │ - Classify resolved inheritance (base/mixin/interface)        │     │
│  │ - Validate inheritance rules                                  │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                               │                                          │
│                               v                                          │
│  Phase 3: Member Completion (existing)                                   │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │ - Topological sort by inheritance                             │     │
│  │ - Copy inherited members                                      │     │
│  │ - Apply mixins                                                │     │
│  │ - Validate interface compliance                               │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                               │                                          │
│                               v                                          │
│  Phase 4: VTable/ITable Building (existing)                              │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │ - Build interface method slots                                │     │
│  │ - Build class vtables                                         │     │
│  │ - Build class itables                                         │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                               │                                          │
│                               v                                          │
│  Phase 5: Build Hash Indexes (NEW)                                       │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │ - Compute TypeHash for all types                              │     │
│  │ - Build hash_to_name reverse index                            │     │
│  │ - Build func_hash_to_name index                               │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Type Resolution

### New TypeResolver for Completion

```rust
// crates/angelscript-compiler/src/passes/completion.rs

use angelscript_core::{
    QualifiedName, UnresolvedType, UnresolvedParam, UnresolvedSignature,
    DataType, Param, MethodSignature, RefModifier, TypeEntry,
};

/// Resolves UnresolvedType to QualifiedName and DataType.
struct TypeResolver<'reg, 'global> {
    registry: &'reg SymbolRegistry,
    global_registry: &'global SymbolRegistry,
}

impl<'reg, 'global> TypeResolver<'reg, 'global> {
    fn new(
        registry: &'reg SymbolRegistry,
        global_registry: &'global SymbolRegistry,
    ) -> Self {
        Self { registry, global_registry }
    }

    /// Resolve an UnresolvedType to a QualifiedName.
    fn resolve_to_name(&self, unresolved: &UnresolvedType) -> Result<QualifiedName, CompilationError> {
        self.resolve_type_name(
            &unresolved.name,
            &unresolved.context_namespace,
            &unresolved.imports,
        ).ok_or_else(|| CompilationError::UnknownType {
            name: unresolved.name.clone(),
            span: Span::default(),
        })
    }

    /// Resolve an UnresolvedType to a DataType.
    fn resolve_to_datatype(&self, unresolved: &UnresolvedType) -> Result<DataType, CompilationError> {
        // Handle void specially
        if unresolved.is_void() {
            return Ok(DataType::void());
        }

        // Handle primitives
        if let Some(dt) = self.try_primitive(&unresolved.name) {
            return Ok(self.apply_modifiers(dt, unresolved));
        }

        // Handle template types (e.g., "array<int>")
        if unresolved.name.contains('<') {
            return self.resolve_template_type(unresolved);
        }

        // Resolve to qualified name
        let qualified_name = self.resolve_to_name(unresolved)?;

        // Look up the type entry
        let entry = self.get_type(&qualified_name)?;
        let type_hash = entry.type_hash();

        // Build DataType with modifiers
        let mut dt = DataType::simple(type_hash);

        // Apply flags based on entry kind
        if entry.as_interface().is_some() {
            dt = dt.with_is_interface(true);
        }
        if entry.as_class().is_some_and(|c| c.is_mixin) {
            dt = dt.with_is_mixin(true);
        }
        if entry.as_enum().is_some() {
            dt = dt.with_is_enum(true);
        }

        Ok(self.apply_modifiers(dt, unresolved))
    }

    /// Apply modifiers from UnresolvedType to DataType.
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
        if unresolved.ref_modifier != RefModifier::None {
            dt = dt.with_ref_modifier(unresolved.ref_modifier);
        }
        dt
    }

    /// Try to resolve a name as a primitive type.
    fn try_primitive(&self, name: &str) -> Option<DataType> {
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

    /// Resolve a type name to QualifiedName using context.
    fn resolve_type_name(
        &self,
        name: &str,
        context_namespace: &[String],
        imports: &[String],
    ) -> Option<QualifiedName> {
        // 1. If already qualified, try direct lookup
        if name.contains("::") {
            let qn = QualifiedName::from_qualified_string(name);
            if self.type_exists(&qn) {
                return Some(qn);
            }
            return None;
        }

        // 2. Try current namespace hierarchy (innermost to outermost)
        for i in (1..=context_namespace.len()).rev() {
            let qn = QualifiedName::new(name, context_namespace[..i].to_vec());
            if self.type_exists(&qn) {
                return Some(qn);
            }
        }

        // 3. Try each import as prefix
        for import in imports {
            let mut ns: Vec<String> = import.split("::").map(|s| s.to_string()).collect();
            let qn = QualifiedName::new(name, ns);
            if self.type_exists(&qn) {
                return Some(qn);
            }
        }

        // 4. Try global namespace
        let qn = QualifiedName::global(name);
        if self.type_exists(&qn) {
            return Some(qn);
        }

        None
    }

    fn type_exists(&self, name: &QualifiedName) -> bool {
        self.registry.contains_type(name) || self.global_registry.contains_type(name)
    }

    fn get_type(&self, name: &QualifiedName) -> Result<&TypeEntry, CompilationError> {
        self.registry.get(name)
            .or_else(|| self.global_registry.get(name))
            .ok_or_else(|| CompilationError::UnknownType {
                name: name.to_string(),
                span: Span::default(),
            })
    }
}
```

### Resolve All Types

```rust
impl<'reg, 'global> TypeCompletionPass<'reg, 'global> {
    /// Phase 1: Resolve all unresolved types.
    fn resolve_all_types(&mut self, output: &mut CompletionOutput) {
        // Resolve class inheritance
        self.resolve_class_inheritance_types(output);

        // Resolve interface base types and methods
        self.resolve_interface_types(output);

        // Resolve funcdef signatures
        self.resolve_funcdef_types(output);

        // Resolve function signatures
        self.resolve_function_types(output);
    }

    fn resolve_class_inheritance_types(&mut self, output: &mut CompletionOutput) {
        let class_names: Vec<QualifiedName> = self.registry
            .classes()
            .map(|c| c.qualified_name.clone())
            .collect();

        for class_name in class_names {
            // Get unresolved inheritance
            let unresolved = {
                let class = self.registry.get(&class_name)
                    .and_then(|e| e.as_class())
                    .unwrap();

                class.inheritance_refs()
                    .filter_map(|r| r.as_unresolved().cloned())
                    .collect::<Vec<_>>()
            };

            // Resolve each
            let resolver = TypeResolver::new(self.registry, self.global_registry);
            for (idx, unresolved_ty) in unresolved.iter().enumerate() {
                match resolver.resolve_to_name(unresolved_ty) {
                    Ok(resolved_name) => {
                        // Determine what kind of type it is
                        if let Some(entry) = resolver.get_type(&resolved_name).ok() {
                            let class = self.registry.get_mut(&class_name)
                                .and_then(|e| e.as_class_mut())
                                .unwrap();

                            if entry.as_interface().is_some() {
                                class.resolve_interface(idx, resolved_name);
                            } else if let Some(base_class) = entry.as_class() {
                                if base_class.is_mixin {
                                    class.resolve_mixin(idx, resolved_name);
                                } else {
                                    class.resolve_base(resolved_name);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        output.errors.push(e);
                    }
                }
            }
        }
    }

    fn resolve_interface_types(&mut self, output: &mut CompletionOutput) {
        let iface_names: Vec<QualifiedName> = self.registry
            .interfaces()
            .map(|i| i.qualified_name.clone())
            .collect();

        for iface_name in iface_names {
            // Resolve base interfaces
            // ... similar to class inheritance

            // Resolve method signatures
            let unresolved_methods = {
                let iface = self.registry.get(&iface_name)
                    .and_then(|e| e.as_interface())
                    .unwrap();
                iface.unresolved_methods.clone()
            };

            let resolver = TypeResolver::new(self.registry, self.global_registry);
            for sig in unresolved_methods {
                match self.resolve_signature(&resolver, &sig) {
                    Ok(resolved) => {
                        let iface = self.registry.get_mut(&iface_name)
                            .and_then(|e| e.as_interface_mut())
                            .unwrap();
                        iface.add_resolved_method(resolved);
                    }
                    Err(e) => {
                        output.errors.push(e);
                    }
                }
            }
        }
    }

    fn resolve_funcdef_types(&mut self, output: &mut CompletionOutput) {
        let funcdef_names: Vec<QualifiedName> = self.registry
            .funcdefs()
            .map(|f| f.qualified_name.clone())
            .collect();

        let resolver = TypeResolver::new(self.registry, self.global_registry);

        for fd_name in funcdef_names {
            let (unresolved_params, unresolved_ret) = {
                let fd = self.registry.get(&fd_name)
                    .and_then(|e| e.as_funcdef())
                    .unwrap();
                (fd.unresolved_params.clone(), fd.unresolved_return_type.clone())
            };

            // Resolve params
            let mut resolved_params = Vec::new();
            for param in &unresolved_params {
                match resolver.resolve_to_datatype(&param.param_type) {
                    Ok(dt) => resolved_params.push(dt),
                    Err(e) => output.errors.push(e),
                }
            }

            // Resolve return type
            match resolver.resolve_to_datatype(&unresolved_ret) {
                Ok(ret) => {
                    let fd = self.registry.get_mut(&fd_name)
                        .and_then(|e| e.as_funcdef_mut())
                        .unwrap();
                    fd.resolve_signature(resolved_params, ret);
                }
                Err(e) => output.errors.push(e),
            }
        }
    }

    fn resolve_function_types(&mut self, output: &mut CompletionOutput) {
        // Get all function qualified names
        let func_names: Vec<QualifiedName> = self.registry
            .functions()
            .map(|f| f.def.qualified_name.clone())
            .collect();

        let resolver = TypeResolver::new(self.registry, self.global_registry);

        for func_name in func_names {
            let funcs = self.registry.get_functions(&func_name).unwrap();

            for (idx, func) in funcs.iter().enumerate() {
                if func.def.is_resolved() {
                    continue; // Already resolved (FFI functions)
                }

                // Resolve params
                let mut resolved_params = Vec::new();
                for unresolved in &func.def.unresolved_params {
                    match resolver.resolve_to_datatype(&unresolved.param_type) {
                        Ok(dt) => {
                            let param = Param::new(&unresolved.name, dt)
                                .with_has_default(unresolved.has_default);
                            resolved_params.push(param);
                        }
                        Err(e) => output.errors.push(e),
                    }
                }

                // Resolve return type
                match resolver.resolve_to_datatype(&func.def.unresolved_return_type) {
                    Ok(ret) => {
                        // Update the function def
                        let funcs_mut = self.registry.get_functions_mut(&func_name).unwrap();
                        funcs_mut[idx].def.resolve_signature(resolved_params, ret);
                    }
                    Err(e) => output.errors.push(e),
                }
            }
        }
    }

    /// Resolve an UnresolvedSignature to a MethodSignature.
    fn resolve_signature(
        &self,
        resolver: &TypeResolver,
        sig: &UnresolvedSignature,
    ) -> Result<MethodSignature, CompilationError> {
        let mut params = Vec::new();
        for param in &sig.params {
            let dt = resolver.resolve_to_datatype(&param.param_type)?;
            params.push(dt);
        }

        let return_type = resolver.resolve_to_datatype(&sig.return_type)?;

        Ok(MethodSignature::new(&sig.name, params, return_type)
            .with_const(sig.is_const))
    }
}
```

---

## Phase 5: Build Hash Indexes

```rust
impl<'reg, 'global> TypeCompletionPass<'reg, 'global> {
    /// Phase 5: Build TypeHash indexes for bytecode generation.
    fn build_hash_indexes(&mut self) {
        self.registry.build_hash_indexes();
    }

    pub fn run(mut self) -> CompletionOutput {
        let mut output = CompletionOutput::default();

        // NEW Phase 1: Resolve all unresolved types
        self.resolve_all_types(&mut output);

        // Phase 2: Classify and validate inheritance (existing)
        // Now works with resolved QualifiedNames

        // Get all script class names
        let class_names: Vec<QualifiedName> = self.registry
            .classes()
            .map(|c| c.qualified_name.clone())
            .collect();

        // Topologically sort classes
        let ordered = match self.topological_sort(&class_names) {
            Ok(ordered) => ordered,
            Err(e) => {
                output.errors.push(e);
                return output;
            }
        };

        // Phase 3: Process each class in order (existing)
        for class_name in &ordered {
            match self.complete_class(class_name, &mut output) {
                Ok(completed) => {
                    if completed {
                        output.classes_completed += 1;
                    }
                }
                Err(e) => output.errors.push(e),
            }
        }

        // Phase 4: Build vtables and itables (existing)
        self.build_interface_method_slots(&mut output);
        self.build_all_vtables(&ordered, &mut output);

        // NEW Phase 5: Build hash indexes
        self.build_hash_indexes();

        output
    }
}
```

---

## Updated TypeCompletionPass Structure

```rust
/// Type Completion Pass - resolves types and finalizes class structures.
pub struct TypeCompletionPass<'reg, 'global> {
    /// Unit registry (mutable for updates)
    registry: &'reg mut SymbolRegistry,
    /// Global registry (read-only for lookups)
    global_registry: &'global SymbolRegistry,
    // REMOVED: pending: PendingResolutions (now stored in entries)
}

impl<'reg, 'global> TypeCompletionPass<'reg, 'global> {
    /// Create a new type completion pass.
    pub fn new(
        registry: &'reg mut SymbolRegistry,
        global_registry: &'global SymbolRegistry,
    ) -> Self {
        Self { registry, global_registry }
    }
}
```

---

## Changes to Existing Code

### Topological Sort

Now uses `QualifiedName` instead of `TypeHash`:

```rust
fn topological_sort(
    &self,
    class_names: &[QualifiedName],
) -> Result<Vec<QualifiedName>, CompilationError> {
    // Build dependency graph using QualifiedName
    let mut graph: FxHashMap<QualifiedName, Vec<QualifiedName>> = FxHashMap::default();

    for name in class_names {
        let class = self.registry.get(name)
            .and_then(|e| e.as_class())
            .unwrap();

        let mut deps = Vec::new();

        // Add base class as dependency
        if let InheritanceRef::Resolved(base_name) = &class.base_class {
            if !base_name.simple_name().is_empty() { // Not void
                deps.push(base_name.clone());
            }
        }

        // Add mixins as dependencies
        for mixin_ref in &class.mixins {
            if let InheritanceRef::Resolved(mixin_name) = mixin_ref {
                deps.push(mixin_name.clone());
            }
        }

        graph.insert(name.clone(), deps);
    }

    // Kahn's algorithm for topological sort
    // ... (same algorithm, different key type)
}
```

### VTable Building

Uses `QualifiedName` for lookups:

```rust
fn build_vtable(&mut self, class_name: &QualifiedName, output: &mut CompletionOutput) {
    // Get base class vtable
    let base_vtable = {
        let class = self.registry.get(class_name)
            .and_then(|e| e.as_class())
            .unwrap();

        if let InheritanceRef::Resolved(base_name) = &class.base_class {
            self.registry.get(base_name)
                .and_then(|e| e.as_class())
                .map(|c| c.vtable.clone())
        } else {
            None
        }
    };

    // Build vtable...
}
```

---

## CompletionOutput Changes

```rust
/// Output of the type completion pass.
#[derive(Debug, Default)]
pub struct CompletionOutput {
    /// Number of types resolved.
    pub types_resolved: usize,  // NEW
    /// Number of function signatures resolved.
    pub signatures_resolved: usize,  // NEW
    /// Number of classes completed.
    pub classes_completed: usize,
    /// Number of methods copied from base classes.
    pub methods_inherited: usize,
    /// Number of properties copied from base classes.
    pub properties_inherited: usize,
    /// Number of classes with vtables built.
    pub vtables_built: usize,
    /// Number of interface itables built.
    pub itables_built: usize,
    /// Collected errors.
    pub errors: Vec<CompilationError>,
}
```

---

## Dependencies

- Phase 1: Core types (`UnresolvedType`, `QualifiedName`)
- Phase 2: Entry types with unresolved fields
- Phase 3: Registry with `QualifiedName` key and `build_hash_indexes()`
- Phase 4: Registration pass storing unresolved types

Phase 6 (Compilation) will use the fully resolved types and hash indexes.
