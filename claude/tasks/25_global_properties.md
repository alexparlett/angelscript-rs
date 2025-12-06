# Task 25: Global Properties

## Overview

Global properties in AngelScript need runtime mutability - values can be added and updated during the program lifecycle. Unlike types and functions which are sealed at compile time, global properties are stored on the `Context` equivalent, not in `FfiRegistry`.

**Prerequisites:** Task 22 (TypeHash Identity System)

---

## Design

### Why Context, Not FfiRegistry?

1. **Runtime mutability**: Global property values can be modified during execution
2. **Dynamic registration**: Properties can be added after initial compilation
3. **Lifetime requirements**: `'app` lifetime for value references requires per-context storage
4. **AngelScript C++ pattern**: Original implementation stores globals on context equivalent

### Registration API (Builder Pattern)

```rust
context.global_property("g_score")
    .data_type("int")        // Type name, resolved via TypeHash
    .value(&mut score)       // Reference to host value
    .namespace("Game")       // Optional namespace
    .const_()                // Optional: mark read-only
    .register()?;

// Shorthand for simple cases
context.global_property("g_count")
    .data_type("int")
    .value(&mut count)
    .register()?;
```

### Storage Structure

```rust
/// A global property exposed to scripts.
pub struct GlobalProperty<'app> {
    /// Property name (without namespace)
    pub name: String,

    /// Fully qualified name (e.g., "Game::g_score")
    pub qualified_name: String,

    /// Resolved type (via TypeHash lookup)
    pub data_type: DataType,

    /// Whether this property is read-only
    pub is_const: bool,

    /// Reference to host application value
    pub value: &'app mut dyn Any,
}
```

### Builder

```rust
pub struct GlobalPropertyBuilder<'ctx, 'app> {
    context: &'ctx mut Context<'app>,
    name: String,
    namespace: Option<String>,
    data_type: Option<String>,  // Type name, resolved on register()
    is_const: bool,
    value: Option<&'app mut dyn Any>,
}

impl<'ctx, 'app> GlobalPropertyBuilder<'ctx, 'app> {
    pub fn data_type(mut self, type_name: &str) -> Self {
        self.data_type = Some(type_name.to_string());
        self
    }

    pub fn value<T: Any>(mut self, val: &'app mut T) -> Self {
        self.value = Some(val);
        self
    }

    pub fn namespace(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.to_string());
        self
    }

    pub fn const_(mut self) -> Self {
        self.is_const = true;
        self
    }

    pub fn register(self) -> Result<(), GlobalPropertyError> {
        // Resolve type via TypeHash
        let type_hash = TypeHash::of(self.data_type.as_ref().unwrap());
        let data_type = self.context.resolve_type(type_hash)?;

        // Build qualified name
        let qualified_name = match &self.namespace {
            Some(ns) => format!("{}::{}", ns, self.name),
            None => self.name.clone(),
        };

        // Create and store property
        let prop = GlobalProperty {
            name: self.name,
            qualified_name,
            data_type,
            is_const: self.is_const,
            value: self.value.unwrap(),
        };

        self.context.add_global_property(prop)
    }
}
```

---

## Lookup Integration

### CompilationContext Changes

`CompilationContext::lookup_global_var` needs to check both FFI and script globals:

```rust
impl<'ast, 'app> CompilationContext<'ast, 'app> {
    pub fn lookup_global_var(&self, name: &str) -> Option<GlobalVarInfo> {
        // 1. Check FFI global properties (from Context)
        if let Some(prop) = self.ffi_globals.get(name) {
            return Some(GlobalVarInfo {
                data_type: prop.data_type.clone(),
                is_const: prop.is_const,
                source: GlobalVarSource::Ffi,
            });
        }

        // 2. Check script global variables (from ScriptRegistry)
        if let Some(var) = self.script_registry.get_global_var(name) {
            return Some(GlobalVarInfo {
                data_type: var.data_type.clone(),
                is_const: var.is_const,
                source: GlobalVarSource::Script,
            });
        }

        None
    }
}
```

---

## Implementation Phases

### Phase 1: Storage Infrastructure
- [ ] Add `GlobalProperty<'app>` struct to `src/ffi/global_property.rs`
- [ ] Add global properties storage to Context
- [ ] Add `GlobalPropertyError` error type

### Phase 2: Builder API
- [ ] Create `GlobalPropertyBuilder` with fluent methods
- [ ] Add `context.global_property(name)` entry point
- [ ] Implement type resolution via TypeHash

### Phase 3: Lookup Integration
- [ ] Update `CompilationContext::lookup_global_var`
- [ ] Add `GlobalVarSource` enum to distinguish FFI vs script globals
- [ ] Wire up to semantic analysis

### Phase 4: Testing
- [ ] Unit tests for builder API
- [ ] Integration tests for FFI global access from scripts
- [ ] Tests for namespace support

---

## Dependencies

- **Task 22 (TypeHash)**: Required for type resolution from string names
  - `TypeHash::of("int")` computes hash from type name
  - Lookup in unified type map without sealed registry requirement

---

## Critical Files

| File | Purpose |
|------|---------|
| `src/ffi/global_property.rs` | GlobalProperty struct and builder |
| `src/ffi/context.rs` | Global property storage |
| `src/semantic/compilation_context.rs` | Lookup integration |

---

## Related Tasks

- **Task 22**: TypeHash Identity System - enables type resolution from names
- **Task 23**: Ergonomic Module API - similar builder patterns
