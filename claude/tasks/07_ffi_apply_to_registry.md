# Task 07: Apply to Registry

**Status:** Not Started
**Depends On:** Tasks 01-06
**Estimated Scope:** Integration with semantic analysis

---

## Objective

Implement the `apply_to_registry()` function that converts FFI registrations into Registry entries for semantic analysis.

## Files to Create/Modify

- `src/ffi/apply.rs` - apply_to_registry() implementation
- Modify `src/semantic/types/registry.rs` - add method to accept FFI definitions

## Key Function

```rust
/// Apply all registered items from a Module to the Registry
pub fn apply_to_registry<'src, 'ast>(
    module: &Module,
    registry: &mut Registry<'src, 'ast>,
    arena: &'ast Bump,
) -> Result<(), ApplyError> {
    // 1. Register types first (so functions can reference them)
    for type_def in &module.types {
        let type_id = registry.register_native_type(type_def)?;
        // Store mapping from type name to TypeId
    }

    // 2. Register enums
    for enum_def in &module.enums {
        registry.register_native_enum(enum_def)?;
    }

    // 3. Register templates
    for template_def in &module.templates {
        registry.register_native_template(template_def)?;
    }

    // 4. Register functions (global and methods)
    for func_def in &module.functions {
        let func_id = registry.register_native_function(func_def)?;
    }

    // 5. Register global properties
    for prop_def in &module.global_properties {
        registry.register_native_global_property(prop_def)?;
    }

    Ok(())
}
```

## Conversions

- `TypeSpec` → `DataType` (resolve type names to TypeId)
- `NativeFunctionDef` → `FunctionDef` (set `is_native: true`)
- `NativeTypeDef` → `TypeDef::Class` or `TypeDef::Interface`
- Parse default parameter expressions into arena

## Implementation Notes

- Type resolution must handle namespaces
- Handle dependencies (method types must exist before methods)
- Parse default argument expressions using the parser
- Validate type references exist

## Acceptance Criteria

- [ ] FFI types appear in Registry.types
- [ ] FFI functions appear in Registry.functions
- [ ] FFI global properties are accessible
- [ ] Type resolution works across namespaces
- [ ] Default arguments are parsed correctly
- [ ] Error messages are helpful for invalid registrations
