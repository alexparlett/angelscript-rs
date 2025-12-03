# Task 08j: Integration & Registry Cleanup

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** All previous subtasks (08a-08i)

---

## Objective

Wire up the built-in modules system and remove the hardcoded implementations from registry.rs.

## Files to Modify

- `src/lib.rs` - Export new modules
- `src/semantic/types/registry.rs` - Remove hardcoded implementations (~800 lines)

## Implementation

### 1. Update src/lib.rs

Add exports for the new modules:

```rust
// In src/lib.rs

// Existing modules...
pub mod ast;
pub mod ffi;
pub mod parser;
pub mod semantic;
// ...

// NEW: Runtime types and built-in modules
pub mod runtime;
pub mod modules;

// Re-export commonly used items
pub use modules::default_modules;
pub use runtime::{ScriptString, ScriptArray, ScriptDict};
```

### 2. Clean Up registry.rs

Remove the following functions and their associated code:

#### Functions to Remove (~800 lines total)

```rust
// REMOVE: String type registration (~400 lines)
fn register_builtin_string(&mut self) { ... }

// REMOVE: String method helpers
fn register_string_length(&mut self) { ... }
fn register_string_substr(&mut self) { ... }
fn register_string_find_first(&mut self) { ... }
fn register_string_find_last(&mut self) { ... }
fn register_string_is_empty(&mut self) { ... }
fn register_string_operators(&mut self) { ... }
// ... etc

// REMOVE: Array template methods (~200 lines)
fn register_array_methods(&mut self, type_id: TypeId, element_type: TypeId) { ... }
fn register_array_length(&mut self) { ... }
fn register_array_resize(&mut self) { ... }
fn register_array_insert_last(&mut self) { ... }
fn register_array_remove_last(&mut self) { ... }
fn register_array_insert_at(&mut self) { ... }
fn register_array_remove_at(&mut self) { ... }
fn register_array_operators(&mut self) { ... }
// ... etc

// REMOVE: Dictionary template methods (~200 lines)
fn register_dictionary_methods(&mut self, type_id: TypeId, key_type: TypeId, value_type: TypeId) { ... }
fn register_dict_set(&mut self) { ... }
fn register_dict_get(&mut self) { ... }
fn register_dict_exists(&mut self) { ... }
fn register_dict_delete(&mut self) { ... }
fn register_dict_is_empty(&mut self) { ... }
fn register_dict_get_size(&mut self) { ... }
fn register_dict_delete_all(&mut self) { ... }
fn register_dict_get_keys(&mut self) { ... }
fn register_dict_operators(&mut self) { ... }
// ... etc
```

#### Keep These Functions

```rust
// KEEP: Primitive type registration
fn register_primitive(&mut self, name: &str, type_id: TypeId) { ... }

// KEEP: Template shell registration (without methods)
fn register_builtin_template(&mut self, name: &str, param_count: usize, type_id: TypeId) { ... }

// KEEP: Placeholder slots for reserved type IDs
fn register_placeholder(&mut self, type_id: TypeId) { ... }

// KEEP: import_modules() - This is what enables FFI modules
pub fn import_modules(&mut self, modules: &[Module<'_>]) -> Result<(), ImportError> { ... }
```

### 3. Update Registry::new()

Modify the initialization to not register hardcoded methods:

```rust
impl Registry {
    pub fn new() -> Self {
        let mut registry = Self {
            types: Vec::new(),
            functions: Vec::new(),
            namespaces: HashMap::new(),
            // ...
        };

        // Register primitive types (TypeIds 0-11)
        registry.register_primitive("void", TypeId::VOID_TYPE);
        registry.register_primitive("bool", TypeId::BOOL_TYPE);
        registry.register_primitive("int8", TypeId::INT8_TYPE);
        registry.register_primitive("int16", TypeId::INT16_TYPE);
        registry.register_primitive("int32", TypeId::INT32_TYPE);
        registry.register_primitive("int64", TypeId::INT64_TYPE);
        registry.register_primitive("uint8", TypeId::UINT8_TYPE);
        registry.register_primitive("uint16", TypeId::UINT16_TYPE);
        registry.register_primitive("uint32", TypeId::UINT32_TYPE);
        registry.register_primitive("uint64", TypeId::UINT64_TYPE);
        registry.register_primitive("float", TypeId::FLOAT_TYPE);
        registry.register_primitive("double", TypeId::DOUBLE_TYPE);

        // Placeholder slots 12-15
        registry.register_placeholder(TypeId::new(12));
        registry.register_placeholder(TypeId::new(13));
        registry.register_placeholder(TypeId::new(14));
        registry.register_placeholder(TypeId::new(15));

        // Register template SHELLS only (no methods - those come from FFI)
        registry.register_builtin_template_shell("string", TypeId::STRING_TYPE);
        registry.register_builtin_template_shell("array", 1, TypeId::ARRAY_TEMPLATE);
        registry.register_builtin_template_shell("dictionary", 2, TypeId::DICT_TEMPLATE);

        // Placeholder slots 19-31
        for i in 19..=31 {
            registry.register_placeholder(TypeId::new(i));
        }

        // NOTE: Methods are now added via import_modules() from FFI

        registry
    }

    /// Register template shell without methods.
    /// Methods are added later via import_modules().
    fn register_builtin_template_shell(&mut self, name: &str, param_count: usize, type_id: TypeId) {
        // Register type definition without methods
        let type_def = TypeDef {
            id: type_id,
            name: name.to_string(),
            kind: TypeKind::Template { param_count },
            methods: Vec::new(),      // Empty - filled by FFI
            properties: Vec::new(),
            behaviors: Behaviors::default(),
            // ...
        };
        self.types.push(type_def);
    }
}
```

### 4. Integration Test

Create a comprehensive integration test:

```rust
// tests/integration/builtin_modules.rs

use angelscript::{Context, default_modules};

#[test]
fn test_default_modules_integration() {
    // Create context with default modules
    let modules = default_modules().expect("default modules should build");

    // Import into registry
    let mut registry = Registry::new();
    registry.import_modules(&modules).expect("import should succeed");

    // Verify string type has methods
    let string_type = registry.get_type(TypeId::STRING_TYPE).unwrap();
    assert!(string_type.methods.iter().any(|m| m.name == "length"));
    assert!(string_type.methods.iter().any(|m| m.name == "substr"));
    assert!(string_type.methods.iter().any(|m| m.name == "findFirst"));

    // Verify array template is registered
    let array_template = registry.get_type(TypeId::ARRAY_TEMPLATE).unwrap();
    assert!(array_template.methods.iter().any(|m| m.name == "length"));
    assert!(array_template.methods.iter().any(|m| m.name == "insertLast"));

    // Verify dictionary template is registered
    let dict_template = registry.get_type(TypeId::DICT_TEMPLATE).unwrap();
    assert!(dict_template.methods.iter().any(|m| m.name == "getSize"));
    assert!(dict_template.methods.iter().any(|m| m.name == "set"));

    // Verify math namespace
    assert!(registry.has_namespace(&["math"]));
    assert!(registry.lookup_function(&["math"], "sin").is_some());
    assert!(registry.lookup_constant(&["math"], "PI").is_some());
}

#[test]
fn test_string_operations_in_script() {
    let ctx = Context::new().expect("context should create");

    let script = r#"
        string s = "hello world";
        int len = s.length();
        string upper = s.toUpper();
        int pos = s.findFirst("world");
    "#;

    let unit = ctx.compile(script).expect("should compile");
    // Verify compilation succeeded - methods resolved correctly
}

#[test]
fn test_array_operations_in_script() {
    let ctx = Context::new().expect("context should create");

    let script = r#"
        array<int> a = {1, 2, 3, 4, 5};
        int len = a.length();
        a.insertLast(6);
        a.sortDesc();
        int first = a[0];  // Should be 6
    "#;

    let unit = ctx.compile(script).expect("should compile");
}

#[test]
fn test_dictionary_operations_in_script() {
    let ctx = Context::new().expect("context should create");

    let script = r#"
        dictionary<string, int> d = {{"one", 1}, {"two", 2}};
        d.set("three", 3);
        int val;
        bool found = d.get("two", val);
        array<string>@ keys = d.getKeys();
    "#;

    let unit = ctx.compile(script).expect("should compile");
}

#[test]
fn test_math_namespace_in_script() {
    let ctx = Context::new().expect("context should create");

    let script = r#"
        double pi = math::PI;
        double s = math::sin(pi / 2);
        double c = math::cos(0);
        double sq = math::sqrt(16);
    "#;

    let unit = ctx.compile(script).expect("should compile");
}
```

### 5. Regression Testing

Run all existing tests to ensure nothing broke:

```bash
cargo test --lib
cargo test --test '*'
```

## Verification Checklist

Before marking complete:

1. [ ] All 1987+ existing tests still pass
2. [ ] `default_modules()` returns 5 modules without error
3. [ ] String type has all methods registered via FFI
4. [ ] Array template has all methods registered via FFI
5. [ ] Dictionary template has all methods registered via FFI
6. [ ] Math namespace has all constants and functions
7. [ ] List initialization syntax works for arrays
8. [ ] List initialization syntax works for dictionaries
9. [ ] Registry.rs is reduced by ~800 lines

## Line Count Verification

Before cleanup:
```bash
wc -l src/semantic/types/registry.rs
# Expected: ~4500 lines
```

After cleanup:
```bash
wc -l src/semantic/types/registry.rs
# Expected: ~3700 lines (reduced by ~800)
```

## Acceptance Criteria

- [ ] `src/lib.rs` exports `runtime` and `modules` modules
- [ ] `default_modules()` exported from lib.rs
- [ ] `ScriptString`, `ScriptArray`, `ScriptDict` exported from lib.rs
- [ ] Registry.rs hardcoded string methods removed (~400 lines)
- [ ] Registry.rs hardcoded array methods removed (~200 lines)
- [ ] Registry.rs hardcoded dictionary methods removed (~200 lines)
- [ ] `Registry::new()` creates shells only (methods from FFI)
- [ ] All existing tests pass
- [ ] New integration tests pass
- [ ] `cargo build --lib` succeeds
- [ ] `cargo test --lib` succeeds
