# Phase 7: Unified Namespace Tree with Unit Isolation

## Overview

Replace the two-registry model (`unit_registry` + `global_registry`) with a single `NamespaceTree` where compilation units are top-level nodes. This simplifies resolution while maintaining unit isolation.

## Problem

The current architecture has:
- `CompilationContext` with two separate registries
- Resolution logic that must search both registries
- Cross-registry `using namespace` directives don't work (e.g., script using FFI namespace)
- Duplicated resolution logic everywhere

## Solution

Single tree with units as top-level nodes:

```
tree_root
├── $ffi/              # FFI-registered types/functions
│   ├── math/
│   │   └── sin, cos, etc.
│   └── string
├── $shared/           # Shared entities (accessible across units)
│   └── ...
├── $unit_0/           # First script compilation unit
│   ├── Game/
│   │   └── Player
│   └── ...
└── $unit_1/           # Second script compilation unit
    └── ...
```

## Key Design Points

### Unit Root vs Tree Root

When compiling a unit, resolution starts from that unit's root:
- `namespace_stack: ["Game", "Entities"]` resolves to `$unit_0/Game/Entities`
- The `CompilationContext` holds `unit_root: NodeIndex` pointing to the unit's subtree

### Resolution Order

At each namespace level (walking up from current to unit root):
1. Check local symbols at current node
2. Follow `Mirrors` edges to check same-named namespace in `$ffi`/`$shared`
3. Follow `Uses` edges to check explicitly imported namespaces
4. Walk up to parent namespace and repeat

The `Mirrors` edges (created automatically) handle FFI/shared visibility. No special fallback logic needed.

### Using Namespace Directives

`using namespace` is purely a script-level concept:
- FFI doesn't have `using` directives
- Stored as `Uses` edges in the tree from the current namespace to the target
- Resolution follows these edges automatically during tree traversal
- No need for separate `imports` field in `CompilationContext`

### Unit Lifecycle

- When a compilation unit is dropped, remove its entire subtree
- Shared/external entities stay in `$shared`
- FFI entities stay in `$ffi`

## Implementation Steps

### Step 1: Edge Types

```rust
pub enum NamespaceEdge {
    Contains(String),  // Parent contains child namespace
    Uses,              // explicit `using namespace` directive
    Mirrors,           // auto-link to same-named namespace in $ffi/$shared
}
```

**Uses**: Created from explicit `using namespace` statements in script code.

**Mirrors**: Created automatically when script declares a namespace that already exists in `$ffi` or `$shared`. Links the script namespace to its FFI/shared counterpart for unified resolution.

When script creates `$unit_0/Math`:
- If `$ffi/Math` exists → add `$unit_0/Math` --Mirrors--> `$ffi/Math`
- If `$shared/Math` exists → add `$unit_0/Math` --Mirrors--> `$shared/Math`

Resolution follows all edge types. Local symbols checked first, then `Mirrors` targets, then `Uses` targets.

### Step 2: Add Unit Management to Tree

```rust
impl NamespaceTree {
    /// Create a new compilation unit, returns its root node
    pub fn create_unit(&mut self, name: &str) -> NodeIndex {
        self.get_or_create_child(self.root, name)
    }

    /// Get the FFI namespace root (creates if needed)
    pub fn ffi_root(&mut self) -> NodeIndex {
        self.get_or_create_child(self.root, "$ffi")
    }

    /// Get the shared namespace root (creates if needed)
    pub fn shared_root(&mut self) -> NodeIndex {
        self.get_or_create_child(self.root, "$shared")
    }

    /// Remove a compilation unit and all its contents
    pub fn remove_unit(&mut self, unit_root: NodeIndex) {
        // Recursively remove all nodes in the subtree
        // ...
    }
}
```

### Step 3: Update SymbolRegistry

```rust
pub struct SymbolRegistry {
    tree: NamespaceTree,
    ffi_root: NodeIndex,
    shared_root: NodeIndex,
    // No more separate registries
}

impl SymbolRegistry {
    pub fn create_unit(&mut self, name: &str) -> NodeIndex {
        self.tree.create_unit(name)
    }

    pub fn ffi_root(&self) -> NodeIndex {
        self.ffi_root
    }

    // Registration methods take unit_root or use ffi_root
    pub fn register_type_in_unit(
        &mut self,
        unit_root: NodeIndex,
        namespace_path: &[&str],
        entry: TypeEntry,
    ) -> Result<(), RegistrationError> {
        // ...
    }
}
```

### Step 4: Update CompilationContext

Use `NodeIndex` directly instead of string paths for efficiency.

**Key insight**: `using namespace` directives become edges in the tree from the current namespace to the imported namespace. No need to store imports separately - the tree's edge traversal handles it.

```rust
pub struct CompilationContext<'a> {
    registry: &'a mut SymbolRegistry,
    unit_root: NodeIndex,              // This unit's root in the tree
    current_namespace: NodeIndex,      // Current position (direct node reference)
    // No imports field needed - uses edges in tree
    // ... rest unchanged
}

impl<'a> CompilationContext<'a> {
    /// Enter a namespace - just follows the tree edge
    pub fn enter_namespace(&mut self, name: &str) {
        self.current_namespace = self.registry
            .tree_mut()
            .get_or_create_child(self.current_namespace, name);
    }

    /// Leave current namespace - walk back to parent
    pub fn leave_namespace(&mut self) {
        if let Some(parent) = self.registry.tree().find_parent(self.current_namespace) {
            self.current_namespace = parent;
        }
    }

    /// Add a using directive - creates an edge in the tree
    pub fn add_using(&mut self, namespace_path: &str) {
        // Resolve namespace path (check unit, $ffi, $shared)
        if let Some(target_node) = self.resolve_namespace_path(namespace_path) {
            // Add Uses edge from current namespace to target
            self.registry.tree_mut().add_using_edge(
                self.current_namespace,
                target_node,
            );
        }
    }

    pub fn resolve_function(&self, name: &str) -> Option<&[FunctionEntry]> {
        let ctx = ResolutionContext {
            current_namespace: self.current_namespace,
        };

        // Tree resolution follows Uses edges automatically
        // Also checks $ffi and $shared as implicit fallbacks
        self.registry.tree().resolve_function(name, &ctx)
    }
}
```

### Step 5: Update Resolution Methods

Resolution walks the namespace hierarchy and follows edges:

```rust
impl NamespaceTree {
    /// Resolve from a starting namespace, walking up and following edges
    pub fn resolve_function_from(
        &self,
        name: &str,
        start: NodeIndex,
    ) -> Option<&[FunctionEntry]> {
        let mut current = Some(start);

        while let Some(node) = current {
            // 1. Check local symbols at this node
            if let Some(result) = self.get_functions_at(node, name) {
                return Some(result);
            }

            // 2. Follow Mirrors edges (same-named namespace in ffi/shared)
            for target in self.mirrors_targets(node) {
                if let Some(result) = self.get_functions_at(target, name) {
                    return Some(result);
                }
            }

            // 3. Follow Uses edges (explicit using namespace)
            for target in self.uses_targets(node) {
                if let Some(result) = self.get_functions_at(target, name) {
                    return Some(result);
                }
            }

            // 4. Walk up to parent namespace
            current = self.parent(node);
        }

        None
    }
}
```

The `Mirrors` edges handle FFI/shared visibility automatically - no separate fallback logic needed.

## Stashed Changes to Incorporate

The git stash `WIP: FunctionEntry refactor` contains key changes that should be incorporated:

### TypeBehaviors stores FunctionEntry (not TypeHash)

```rust
pub struct TypeBehaviors {
    pub constructors: Vec<FunctionEntry>,
    pub factories: Vec<FunctionEntry>,
    pub destructor: Option<FunctionEntry>,
    pub addref: Option<FunctionEntry>,
    pub release: Option<FunctionEntry>,
    pub operators: FxHashMap<OperatorBehavior, Vec<FunctionEntry>>,
    // ...
}
```

### ClassEntry.methods stores FunctionEntry

```rust
pub struct ClassEntry {
    pub methods: FxHashMap<String, Vec<FunctionEntry>>,
    // ...
}
```

### Overload resolution takes &[&FunctionEntry]

```rust
pub fn resolve_overload(
    candidates: &[&FunctionEntry],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OverloadMatch, CompilationError>
```

### VTable keeps TypeHash (for runtime dispatch)

VTable stays as-is with TypeHash - it's for runtime polymorphic dispatch, not compile-time resolution.

## Additional Changes Needed

### Type Aliases as Part of TypeEntry

Currently type aliases are separate. Consider making them a variant of TypeEntry:

```rust
pub enum TypeEntry {
    Primitive(PrimitiveEntry),
    Class(ClassEntry),
    Interface(InterfaceEntry),
    Enum(EnumEntry),
    Funcdef(FuncdefEntry),
    TemplateParam(TemplateParamEntry),
    Alias { target: TypeHash, name: String },  // NEW
}
```

### Remove Deprecated Hash-Based Methods

After this phase, remove all deprecated `get_*` methods that take `TypeHash` for lookup. All lookups should go through the tree with proper scope resolution.

## Testing Strategy

1. Unit tests for tree manipulation (create/remove units)
2. Resolution tests with cross-unit visibility
3. `using namespace` tests across unit boundaries
4. Shared entity visibility tests
5. Unit cleanup tests (ensure proper removal)

## Migration Path

1. Implement unified tree structure
2. Update SymbolRegistry to use single tree
3. Update CompilationContext
4. Update all resolution call sites
5. Remove old two-registry code
6. Apply stashed FunctionEntry changes
7. Remove deprecated methods

## Dependencies

- Phase 6 (NamespaceTree storage) - COMPLETED
- Stashed changes from FunctionEntry refactor

## Success Criteria

- [ ] Single NamespaceTree with unit isolation
- [ ] `using namespace` works across FFI/script boundaries
- [ ] Unit removal cleans up all associated data
- [ ] All resolution goes through tree (no hash-based fallbacks)
- [ ] FunctionEntry stored directly (no hash lookups for overload resolution)
- [ ] All existing tests pass
