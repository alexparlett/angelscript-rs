# Task 06: Template Builder

**Status:** MERGED INTO TASK 04

---

## Note

This task has been merged into [Task 04 (Class Builder)](04_ffi_class_builder.md).

Templates are now registered using `register_type` with `<class T>` syntax, following the AngelScript C++ convention. The template functionality is unified with regular type registration.

## Example

```rust
// Register template with <class T> syntax
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .template_callback(|info| TemplateValidation::valid())?
    .factory("array<T>@ f()", || ScriptArray::new())?
    .method("void insertLast(const T &in)", array_insert_last)?
    .build()?;
```

See Task 04 for full documentation of template type registration.
