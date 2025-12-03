# Task 07: Apply to Registry

**Status:** ✅ Complete
**Depends On:** Tasks 01-06
**Completed:** 2025-12-03

---

## Objective

Implement `Registry::import_modules()` method that converts FFI registrations from multiple `Module`s into Registry entries for semantic analysis.

## Implementation

Added `import_modules(&mut self, modules: &[Module])` method to `Registry` in `src/semantic/types/registry.rs`.

### Method Signature

```rust
impl<'ast> Registry<'ast> {
    pub fn import_modules(&mut self, modules: &[Module<'_>]) -> Result<(), ImportError>
}
```

### Error Type

```rust
#[derive(Debug, Clone, Error)]
pub enum ImportError {
    #[error("type not found: {0}")]
    TypeNotFound(String),

    #[error("duplicate type: {0}")]
    DuplicateType(String),

    #[error("type resolution failed for '{type_name}': {reason}")]
    TypeResolutionFailed { type_name: String, reason: String },
}
```

### Processing Order

Uses correct dependency ordering with two-pass type import:

1. **Enums** - No dependencies, register first
2. **Interfaces** - Abstract method signatures only
3. **Funcdefs** - Function pointer types
4. **Types (shell)** - Register type name so it can be looked up
5. **Types (details)** - Fill in methods, operators, properties (may reference other types)
6. **Functions** - Global functions, may reference any type
7. **Global Properties** - May reference any type

The two-pass type import handles circular references between types in the same module.

### Type Resolution

Added `resolve_ffi_type_expr()` helper to convert AST `TypeExpr` to semantic `DataType`:
- Primitives → Fixed TypeIds (int → INT32_TYPE, etc.)
- Named types → Registry lookup
- Template types → Registry instantiation
- Type modifiers (const, @, &in/out/inout)

## Files Modified

- `src/semantic/types/registry.rs` - Added `import_modules()` and supporting code (~300 lines)
- `src/semantic/types/mod.rs` - Re-export `ImportError`
- `src/semantic/mod.rs` - Re-export `ImportError`

## Tests Added

13 new tests covering:
- Import empty module
- Import module with enum
- Import module with interface
- Import module with funcdef
- Import module with class (methods, operators, properties)
- Import module with global function
- Import module with global property
- Type resolution for various type expressions
- Error cases (duplicate types, unknown types)
- Multiple module imports
- Namespace handling

## Acceptance Criteria

- [x] FFI types appear in Registry.types
- [x] FFI functions appear in Registry.functions
- [x] FFI global properties are accessible
- [x] Type resolution works across namespaces
- [x] Default arguments are parsed correctly
- [x] Error messages are helpful for invalid registrations
