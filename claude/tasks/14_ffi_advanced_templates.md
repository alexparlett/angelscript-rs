# Task 14: Advanced Template Features

**Status:** Not Started
**Depends On:** Tasks 01-11
**Phase:** Post-Migration

---

## Objective

Add support for advanced template features to the FFI system:
- `if_handle_then_const` keyword for proper const semantics with handles
- Child funcdefs (template-dependent function pointer types)
- Template specializations (optimized implementations for specific types)

## Background

AngelScript has several advanced template features beyond basic `<class T>` templates:

### 1. `if_handle_then_const` Keyword

When a template method takes `const T&in` and T is a handle type, only the handle becomes const, not the referenced object. The `if_handle_then_const` keyword fixes this:

```cpp
// Without: array<Obj@>.find() can't accept const Obj@
r = engine->RegisterObjectMethod("array<T>",
    "int find(const T&in value) const", ...);

// With: array<Obj@>.find() accepts const Obj@const
r = engine->RegisterObjectMethod("array<T>",
    "int find(const T&in if_handle_then_const value) const", ...);
```

### 2. Child Funcdefs

Funcdefs that belong to a template type, with signatures that depend on template parameters:

```cpp
r = engine->RegisterFuncdef("bool myTemplate<T>::callback(const T &in)");
r = engine->RegisterObjectMethod("myTemplate<T>",
    "void doCallback(const callback &in)", ...);
```

### 3. Template Specializations

Register optimized implementations for specific type arguments:

```cpp
// Generic template
r = engine->RegisterObjectType("myTemplate<T>", 0, asOBJ_REF | asOBJ_TEMPLATE);

// Specialization for float - no hidden type parameter
r = engine->RegisterObjectType("myTemplate<float>", 0, asOBJ_REF);
r = engine->RegisterObjectBehaviour("myTemplate<float>",
    asBEHAVE_FACTORY, "myTemplate<float>@ f()", ...);
```

## Design

### 1. `if_handle_then_const` Support

Parser recognizes the keyword after `&in`:

```rust
// Declaration string
"int find(const T&in if_handle_then_const value) const"

// Stored in FunctionParam
pub struct FunctionParam<'ast> {
    // ... existing fields
    pub if_handle_then_const: bool,  // NEW
}
```

Usage in FFI:
```rust
module.register_type::<ScriptArray>("array<class T>")
    .method("int find(const T&in if_handle_then_const value) const", array_find)?
    .build()?;
```

### 2. Child Funcdef Support

New method on ClassBuilder for registering child funcdefs:

```rust
module.register_type::<Container>("Container<class T>")
    .reference_type()
    .funcdef("bool callback(const T &in)")?  // Registers Container<T>::callback
    .method("void forEach(const callback &in cb)", container_foreach)?
    .build()?;
```

Storage:
```rust
pub struct NativeTypeDef<'ast> {
    // ... existing fields
    pub funcdefs: Vec<NativeFuncdef<'ast>>,  // NEW - child funcdefs
}

pub struct NativeFuncdef<'ast> {
    pub name: Ident<'ast>,
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
}
```

### 3. Template Specialization Support

Register specializations as separate types with concrete type arguments:

```rust
// Register generic template
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .template_callback(|_| TemplateValidation::valid())?
    .build()?;

// Register specialization for int - uses different Rust type
module.register_type::<IntArray>("array<int>")
    .reference_type()
    .factory("array<int>@ f()", IntArray::new)?
    .method("void insertLast(int value)", IntArray::push)?
    .build()?;
```

The registry detects that `array<int>` has concrete type arguments and treats it as a specialization rather than a template.

## Implementation Steps

1. **Parser**: Add `if_handle_then_const` keyword recognition after `&in`
2. **FunctionParam**: Add `if_handle_then_const` field
3. **ClassBuilder**: Add `funcdef()` method
4. **Storage**: Add `funcdefs` to `NativeTypeDef`
5. **Apply**: Handle child funcdef registration in Registry
6. **Specialization**: Detect concrete type arguments in `register_type` name
7. **Registry**: Store specializations separately, prefer over generic template
8. **Tests**: Cover all three features

## Files to Modify

- `src/ast/decl_parser.rs` - Parse `if_handle_then_const` keyword
- `src/ast/types.rs` - Add field to FunctionParam
- `src/ffi/class.rs` - Add `funcdef()` method
- `src/ffi/types.rs` - Add funcdefs to NativeTypeDef
- `src/ffi/apply.rs` - Handle child funcdefs and specializations
- `src/semantic/types/registry.rs` - Store and resolve specializations

## Acceptance Criteria

- [ ] Parser handles `if_handle_then_const` keyword in parameter declarations
- [ ] `FunctionParam` stores `if_handle_then_const` flag
- [ ] `ClassBuilder::funcdef()` registers template-dependent funcdefs
- [ ] Child funcdefs are accessible as `TemplateName<T>::funcdefName`
- [ ] Specializations registered with concrete types (e.g., `array<int>`)
- [ ] Registry prefers specialization over generic template when matching
- [ ] Tests cover all three advanced template features

## Script Usage

```angelscript
// if_handle_then_const - can pass const handle
array<const Obj@> arr;
Obj@ obj = getObj();
int idx = arr.find(obj);  // Works with if_handle_then_const

// Child funcdef
Container<int> c;
Container<int>::callback@ cb = function(x) { return x > 0; };
c.forEach(cb);

// Specialization - uses optimized int array
array<int> intArr;  // Uses IntArray specialization
intArr.insertLast(42);
```
