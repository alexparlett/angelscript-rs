# Task 09: Update Entry Points (Benches and Integration Tests)

**Status:** Not Started
**Depends On:** Tasks 07, 08
**Estimated Scope:** API migration

---

## Objective

Update benchmarks and integration tests to use the new Context/Unit API instead of ScriptModule directly. This provides a higher-level entry point that includes FFI registration.

## Files to Modify

- `benches/module_benchmarks.rs`
- `tests/module_tests.rs`
- `tests/test_harness.rs`

## Current API

```rust
// Current: Direct ScriptModule usage
let mut module = ScriptModule::new();
module.add_source("test.as", source)?;
module.build()?;
```

## New API

```rust
// New: Context → Unit flow with FFI
let ctx = Context::with_default_modules()?;
let mut unit = ctx.create_unit();
unit.add_source("test.as", source)?;
unit.build()?;
```

## Benchmark Updates

```rust
// benches/module_benchmarks.rs
fn bench_build_hello_world(c: &mut Criterion) {
    let source = include_str!("../test_scripts/hello_world.as");
    let ctx = Context::with_default_modules().unwrap();

    c.bench_function("build_hello_world", |b| {
        b.iter(|| {
            let mut unit = ctx.create_unit();
            unit.add_source("hello_world.as", source).unwrap();
            unit.build().unwrap();
            black_box(unit)
        });
    });
}
```

## Integration Test Updates

```rust
// tests/module_tests.rs
fn build_script(filename: &str) -> Unit {
    let ctx = Context::with_default_modules().unwrap();
    let mut unit = ctx.create_unit();
    unit.add_source(filename, load_script(filename)).expect("Failed to add source");
    unit.build().expect("Failed to build");
    unit
}

#[test]
fn test_hello_world() {
    let unit = build_script("hello_world.as");
    assert!(unit.is_built());
    assert!(unit.function_count() >= 1);
}
```

## What to Keep

- ScriptModule can remain as a lower-level API
- Or ScriptModule becomes an alias/wrapper around Unit
- Existing test assertions should still pass

## Additional Integration Tests

Add test scripts and integration tests for FFI template types with mixed concrete and template parameters:

### test_scripts/ffi_templates.as
```angelscript
// Test FFI template types with mixed concrete/template params
void testStringMap() {
    // stringmap<string, class T> - key is always string, value is template param
    stringmap<int> intMap;
    stringmap<float> floatMap;

    intMap.set("count", 42);
    floatMap.set("pi", 3.14);
}
```

### tests/module_tests.rs
```rust
#[test]
fn test_ffi_mixed_template_params() {
    // Register stringmap<string, class T> where string is concrete, T is template param
    let ctx = Context::new();
    ctx.register_type::<StringMap>("stringmap<string, class T>")
        .reference_type()
        .template_callback(|_| TemplateValidation::valid())
        .build()
        .unwrap();

    let mut unit = ctx.create_unit();
    unit.add_source("test.as", "void main() { stringmap<int> m; }").unwrap();
    unit.build().expect("Mixed template params should compile");
}
```

## Acceptance Criteria

- [ ] All benchmarks use Context/Unit API
- [ ] All integration tests use Context/Unit API
- [ ] Benchmarks still measure the same operations
- [ ] Performance is not regressed significantly
- [ ] Test coverage remains the same
- [ ] Integration test validates FFI templates with mixed concrete/template parameters
