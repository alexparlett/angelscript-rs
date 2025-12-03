# Task 19: FFI Import System Review & Test Migration

**Priority**: TOP PRIORITY
**Status**: In Progress

## Overview

Comprehensive review and verification of the FFI import system in `Registry`, migration of all unit tests to use the new module-based approach (`compile_with_modules`), and ensuring feature parity with the previously hardcoded implementations.

## Background

The project has transitioned from hardcoded type implementations (STRING_TYPE, ARRAY_TEMPLATE, DICT_TEMPLATE constants) to a dynamic FFI module system. This transition requires:

1. **Verification** that the FFI import system works correctly for all import categories
2. **Migration** of all unit tests to use `compile_with_modules` with appropriate modules
3. **Feature parity** ensuring the new system provides all functionality that existed before

## Goals

### Goal 1: Complete FFI Import System Review

Review and verify each import function in `src/semantic/types/registry.rs`:

| Import Function | Lines | What It Does | Status |
|----------------|-------|--------------|--------|
| `import_enum` | ~1477-1496 | Creates TypeDef::Enum | ⬜ Needs Review |
| `import_interface` | ~1499-1529 | Creates TypeDef::Interface with method signatures | ⬜ Needs Review |
| `import_funcdef` | ~1532-1567 | Creates TypeDef::Funcdef | ⬜ Needs Review |
| `import_type_shell` | ~1569-1647 | Creates TypeDef::Class shell + TemplateParams | ⬜ Needs Review |
| `import_behaviors` | ~1649-1772 | Creates TypeBehaviors entries | ⬜ Needs Review |
| `import_type_details` | ~1777-1878 | Populates methods/operators/properties | ⬜ Needs Review |
| `import_method` | ~1883-1934 | Creates FunctionDef for method | ⬜ Needs Review |
| `import_property_with_template` | ~1946-2006 | Creates getter/setter FunctionDefs | ⬜ Needs Review |
| `import_function` | ~2008-2050 | Creates FunctionDef for global function | ⬜ Needs Review |
| `import_global_property` | ~2052-2063 | Creates GlobalVarDef | ⬜ Needs Review |
| `convert_interface_method` | ~2066-2088 | Creates MethodSignature | ⬜ Needs Review |
| Type resolution functions | ~2090-2290 | resolve_ffi_*_type functions | ⬜ Needs Review |

### Goal 2: Verify Template System

Template parameters and instantiation are critical and currently **UNVERIFIED**:

1. **Template Parameter Registration**
   - Template params registered as `TypeDef::TemplateParam`
   - Naming convention: `"<type>::$<param>"` (e.g., `"array::$T"`)
   - Verify params have correct TypeIds and can be resolved

2. **Template Method Import**
   - Methods imported during `import_type_details` phase
   - Method signatures should reference TemplateParam TypeIds for `T`
   - Verify method signatures are correct (not void/empty)

3. **Template Instantiation**
   - Happens at script compile time in `resolve_type_expr` (type_compilation.rs)
   - `instantiate_template` specializes methods via `specialize_function`
   - Verify `array<int>` has working `length()`, `insertAt()`, etc.

### Goal 3: Migrate Unit Tests

All tests currently using deprecated `compile()` must migrate to `compile_with_modules()`:

```rust
// OLD (deprecated)
let result = Compiler::compile(&script);

// NEW
let result = Compiler::compile_with_modules(&script, &[
    &array_module(),
    &string_module(),
]);
```

Tests to migrate (organized by module):

#### src/semantic/passes/function_processor/*.rs
- [ ] Identify all tests using `compile()`
- [ ] Determine which FFI modules each test needs
- [ ] Update to `compile_with_modules`

#### src/semantic/passes/type_compilation.rs
- [ ] Tests that use array types
- [ ] Tests that use string types

#### Other test files
- [ ] Search all test files for `compile(` calls
- [ ] Migrate each to appropriate module set

### Goal 4: Feature Parity Verification

Ensure the new FFI modules provide everything that existed before:

#### String Type (`string_module`)
- [x] Basic string operations (length, isEmpty, etc.)
- [x] Operators (=, +, +=, ==, !=, <, >, <=, >=, [])
- [x] Methods (substr, findFirst, findLast, insert, erase, etc.)
- [x] Global functions (parseInt, parseFloat, formatInt, formatFloat, etc.)

#### Array Template (`array_module`)
- [ ] Template with parameter T
- [ ] Constructors (default, sized, list initialization)
- [ ] Length and resize operations
- [ ] Element access ([])
- [ ] Insert/remove operations
- [ ] Sort and find operations
- [ ] List behaviors (list_factory, list_construct)

#### Dictionary Template (future - `dict_module`)
- [ ] Template with key/value parameters
- [ ] Get/set operations
- [ ] Key enumeration
- [ ] Size operations

## Immediate Actions

### Step 1: Remove Debug Logging (FIRST)
Remove `eprintln!` debug statements from registry.rs (lines ~2252-2267):
```rust
// DELETE these lines:
eprintln!("DEBUG: Looking up template param...");
eprintln!("DEBUG: Available types...");
eprintln!("DEBUG: Fallback lookup...");
eprintln!("DEBUG: Unqualified lookup...");
```

### Step 2: Run Current Tests
```bash
cargo test --lib
```
Capture current failure count and identify specific failures.

### Step 3: Write Verification Tests
Create focused tests to verify the import system:

```rust
#[test]
fn test_template_param_registration() {
    let registry = Registry::new();
    registry.import_modules(&[&array_module()]);

    // Verify array::$T exists and is a TemplateParam
    let t_id = registry.type_by_name.get("array::$T");
    assert!(t_id.is_some());

    let t_def = registry.get_type(*t_id.unwrap());
    assert!(matches!(t_def, TypeDef::TemplateParam { .. }));
}

#[test]
fn test_template_method_import() {
    let registry = Registry::new();
    registry.import_modules(&[&array_module()]);

    // Get array template
    let array_id = registry.type_by_name.get("array").unwrap();
    let array_def = registry.get_type(*array_id);

    // Verify methods exist and have correct signatures
    if let TypeDef::Class { method_ids, .. } = array_def {
        assert!(!method_ids.is_empty(), "Array should have methods");

        // Check length() method exists with correct return type
        // ...
    }
}

#[test]
fn test_template_instantiation() {
    let script = parse("array<int> a; int len = a.length();");
    let result = Compiler::compile_with_modules(&script, &[&array_module()]);

    assert!(result.errors.is_empty(), "Should compile without errors");
}
```

### Step 4: Fix Identified Issues
Based on test results, fix any broken import functions.

### Step 5: Migrate Remaining Tests
Update all tests to use `compile_with_modules`.

## Success Criteria

1. **All import functions verified** with focused unit tests
2. **Template system working**:
   - `array<int>` compiles successfully
   - Methods like `length()`, `insertAt()` are callable
   - Operators like `[]` work
3. **All tests pass** using `compile_with_modules`
4. **No deprecated `compile()` calls** remaining in test code
5. **Feature parity confirmed** - all previous functionality works

## Files to Modify

| File | Changes |
|------|---------|
| `src/semantic/types/registry.rs` | Remove debug logging, fix any import bugs |
| `src/semantic/passes/function_processor/*.rs` | Migrate tests to compile_with_modules |
| `src/semantic/passes/type_compilation.rs` | Migrate tests to compile_with_modules |
| `src/semantic/compiler.rs` | Potentially remove deprecated compile() after migration |
| `src/modules/*.rs` | Fix any module registration issues found |

## Related Tasks

- Task 08: FFI & Builtin Modules (provides context for module implementations)
- Previous work removed STRING_TYPE, ARRAY_TEMPLATE, DICT_TEMPLATE constants

## Notes

- This task was created because claims about the import system "working correctly" were based on code inspection, not actual verification
- The `TypeNotFound("T")` error from previous sessions suggests potential issues
- ~20 tests were failing at last count - these need investigation
