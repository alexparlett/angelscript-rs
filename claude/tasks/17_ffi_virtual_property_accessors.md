# Task 17: Virtual Property Accessors

**Status:** Not Started
**Depends On:** Tasks 01-07
**Phase:** Integration

---

## Objective

Add support for virtual property accessors - methods registered with the `property` keyword that are exposed to scripts as if they were properties.

## Background

AngelScript supports virtual property accessors where getter/setter methods are exposed as properties:

```cpp
// C++ registration
engine->RegisterObjectMethod("MyClass", "int get_count() const property", ...);
engine->RegisterObjectMethod("MyClass", "void set_count(int) property", ...);

// Global virtual property accessors
engine->RegisterGlobalFunction("int get_globalScore() property", ...);
engine->RegisterGlobalFunction("void set_globalScore(int) property", ...);
```

Script code can then access these as properties:
```angelscript
MyClass obj;
obj.count = 5;      // Calls set_count(5)
int x = obj.count;  // Calls get_count()

globalScore = 100;  // Calls set_globalScore(100)
int s = globalScore; // Calls get_globalScore()
```

Works the same as script class virtual properties: https://angelcode.com/angelscript/sdk/docs/manual/doc_script_class_prop.html

## Design

### API

```rust
// Class virtual property accessors
module.register_type::<Counter>("Counter")
    .value_type()
    .method("int get_count() const property", Counter::get_count)?
    .method("void set_count(int) property", Counter::set_count)?
    .build()?;

// Global virtual property accessors
module.register_fn("int get_score() property", get_score)?;
module.register_fn("void set_score(int) property", set_score)?;
```

### Key Design Decisions

1. **`property` keyword is required** - explicitly marks accessor methods
2. **Stored as regular methods** - no separate property storage needed
3. **Naming convention**: `get_<name>` for getter, `set_<name>` for setter
4. **Semantic analysis unchanged** - already resolves `obj.count` to `get_count()`/`set_count()`
5. **`ClassBuilder::property()` stays** - for direct field access (different use case)
6. **Remove `ClassBuilder::property_get()`** - replaced by method with `property` keyword

### Registry Integration

When `import_modules()` processes methods with `property` keyword:
- Register as normal method with `is_native: true`
- Method name includes `get_`/`set_` prefix
- No additional metadata needed - naming convention is sufficient

### Parser Changes

The `property` keyword needs to be recognized after function attributes:
```
"int get_count() const property"
                       ^^^^^^^^ parse as function attribute
```

## Validation Rules

When `property` keyword is present:
- Getter: name must start with `get_`, no params, returns non-void, const optional
- Setter: name must start with `set_`, exactly one param (value), returns void
- **No type matching validation** between getter return type and setter param type - application developer's responsibility

**Note:** Indexed property accessors are not currently supported in script classes, so not included in FFI.

## Implementation Steps

1. Add `property` keyword parsing to `Parser::function_decl()`
2. Store `is_property` flag in `FunctionSignatureDecl`
3. Remove `property_get()` from ClassBuilder
4. Validate property accessor naming in `method()` when `property` keyword present
5. Update tests

## Files to Modify

- `src/ast/decl_parser.rs` - Parse `property` keyword in function signatures
- `src/ffi/class_builder.rs` - Remove `property_get()`, detect `property` keyword in `method()`
- `src/ffi/module.rs` - Detect `property` keyword in `register_fn()`
- `src/ffi/apply.rs` - No changes needed (methods registered normally)

## Acceptance Criteria

- [ ] Parser recognizes `property` keyword after function signature
- [ ] `FunctionSignatureDecl` has `is_property` flag
- [ ] `ClassBuilder::property_get()` removed
- [ ] `ClassBuilder::method()` validates `get_`/`set_` naming when `property` keyword present
- [ ] `Module::register_fn()` validates `get_`/`set_` naming when `property` keyword present
- [ ] Tests cover: class virtual props, global virtual props, read-only (getter only), read-write (getter + setter)
