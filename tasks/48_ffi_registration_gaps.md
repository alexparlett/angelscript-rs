# Task 48: FFI Registration Gaps

## Problem Summary

The FFI registration system has significant gaps where the design was partially implemented but never completed. The macro-generated metadata exists, but:

1. Native functions are never captured/wrapped
2. Rust `TypeId` is never stored for type verification
3. Behaviors are never wired to `TypeBehaviors`
4. Template callbacks are never registered
5. Tests verify what was written, not what was required

## Gaps Identified

### 1. ClassMeta Missing `rust_type_id`

**Design:** Types should store their Rust `TypeId` for runtime type verification when values pass between Rust and script.

**Current:**
```rust
pub struct ClassMeta {
    pub name: &'static str,
    pub type_hash: TypeHash,
    pub type_kind: TypeKind,
    // ... no rust_type_id
}
```

**Fix:** Add `rust_type_id: Option<TypeId>` field.

### 2. FunctionMeta Missing `native_fn`

**Design (from type_registry_design.md:842):**
```rust
FunctionMeta {
    name: "opAdd",
    native_fn: Box::new(|...| ...),  // Was in design!
    // ...
}
```

**Current:** Only metadata, no callable.

**Fix:** Add `native_fn: Option<NativeFn>` field. `NativeFn` already uses `Arc` internally and is `Clone`.

### 3. Function Macro Doesn't Wrap Functions

**Design:** Macro should generate a `NativeFn` wrapper around the original function.

**Current:** Macro only generates metadata, original function is either renamed (unit struct pattern) or kept (keep pattern) but never wrapped.

**Fix:** Generate wrapper closure in macro that:
- Captures `&self`/`&mut self` for methods
- Extracts typed arguments from `CallContext`
- Calls original function
- Converts return value back to `VmSlot`

### 4. install_function Ignores Behaviors

**Current (context.rs:287-290):**
```rust
let (is_constructor, is_destructor) = match &meta.behavior {
    Some(Behavior::Constructor) => (true, false),
    Some(Behavior::Destructor) => (false, true),
    _ => (false, false),  // ALL OTHER BEHAVIORS IGNORED
};
```

**Missing:** All other behaviors should populate `ClassEntry.behaviors`:
- `Constructor` → `behaviors.constructors.push(hash)`
- `Factory` → `behaviors.factories.push(hash)`
- `Destructor` → `behaviors.destructor = Some(hash)`
- `AddRef` → `behaviors.addref = Some(hash)`
- `Release` → `behaviors.release = Some(hash)`
- `ListConstruct` → `behaviors.list_construct = Some(hash)`
- `ListFactory` → `behaviors.list_factory = Some(hash)`
- `TemplateCallback` → `behaviors.template_callback = Some(hash)` + register callback
- `Operator(op)` → `behaviors.operators[op].push(hash)`
- GC behaviors → respective fields

### 5. Template Callbacks Never Registered

**Current:** `Behavior::TemplateCallback` is recognized but:
- Never registered to `registry.template_callbacks`
- Never linked to `ClassEntry.behaviors.template_callback`
- The actual callback function is never captured

**Fix:**
1. Capture callback in `FunctionMeta.native_fn`
2. In `install_function`, when behavior is `TemplateCallback`:
   - Create `TemplateCallback` wrapper from `NativeFn`
   - Register to `registry.register_template_callback(type_hash, callback)`
   - Set `class_entry.behaviors.template_callback = Some(func_hash)`

### 6. install_function Uses Wrong FunctionEntry Constructor

**Current:**
```rust
let entry = FunctionEntry::ffi(def);  // FunctionImpl::Native(None)
```

**Should be:**
```rust
let entry = FunctionEntry::ffi_with_native(def, meta.native_fn.unwrap());
```

## Implementation Plan

### Phase 1: Core Type Changes

#### 1.1 Add `rust_type_id` to ClassMeta
```rust
// crates/angelscript-core/src/meta.rs
pub struct ClassMeta {
    pub name: &'static str,
    pub type_hash: TypeHash,
    pub type_kind: TypeKind,
    pub rust_type_id: Option<TypeId>,  // NEW
    pub properties: Vec<PropertyMeta>,
    // ...
}
```

#### 1.2 Add `native_fn` to FunctionMeta
```rust
// crates/angelscript-core/src/meta.rs
pub struct FunctionMeta {
    pub name: &'static str,
    pub native_fn: Option<NativeFn>,  // NEW
    pub params: Vec<ParamMeta>,
    // ...
}
```

#### 1.3 Update TypeSource Usage
```rust
// src/context.rs - install_class
TypeSource::Ffi { rust_type_id: meta.rust_type_id }
```

### Phase 2: Macro Changes

#### 2.1 Update `#[derive(Any)]` Macro
Generate `rust_type_id: Some(std::any::TypeId::of::<Self>())` in `ClassMeta`.

#### 2.2 Update `#[function]` Macro

This is the complex part. The macro needs to generate a `NativeFn` wrapper.

**For free functions:**
```rust
#[function]
fn add(a: i32, b: i32) -> i32 { a + b }

// Generates:
fn __as_fn__add(a: i32, b: i32) -> i32 { a + b }

struct add;
impl HasFunctionMeta for add {
    fn __as_fn_meta() -> FunctionMeta {
        FunctionMeta {
            name: "add",
            native_fn: Some(NativeFn::new(|ctx: &mut CallContext| {
                let a = ctx.arg::<i32>(0)?;
                let b = ctx.arg::<i32>(1)?;
                let result = __as_fn__add(a, b);
                ctx.set_return(result);
                Ok(())
            })),
            params: vec![...],
            // ...
        }
    }
}
```

**For methods:**
```rust
impl Player {
    #[function(instance, const)]
    fn get_health(&self) -> i32 { self.health }
}

// Generates:
impl Player {
    fn get_health(&self) -> i32 { self.health }

    fn __get_health_meta_fn() -> FunctionMeta {
        FunctionMeta {
            name: "get_health",
            native_fn: Some(NativeFn::new(|ctx: &mut CallContext| {
                let this = ctx.this::<Player>()?;
                let result = this.get_health();
                ctx.set_return(result);
                Ok(())
            })),
            // ...
        }
    }

    const get_health__meta: fn() -> FunctionMeta = Self::__get_health_meta_fn;
}
```

**Challenge:** The macro needs to generate code that:
1. Knows how to extract each parameter type from `CallContext`
2. Knows how to convert the return type back
3. Handles `&self`, `&mut self`, and no-self cases

This requires `FromScript` and `ToScript` traits (or similar) that the macro can rely on.

### Phase 3: Context Installation Changes

#### 3.1 Update install_function for Behaviors

```rust
fn install_function(&mut self, ..., meta: FunctionMeta) -> Result<(), ContextError> {
    // ... existing code to build FunctionDef ...

    // Use native_fn if present
    let entry = if let Some(native_fn) = meta.native_fn {
        FunctionEntry::ffi_with_native(def, native_fn)
    } else {
        FunctionEntry::ffi(def)
    };

    self.registry.register_function(entry)?;

    // Wire behaviors to type
    if let Some(object_type) = meta.associated_type {
        if let Some(behavior) = &meta.behavior {
            self.wire_behavior(object_type, func_hash, behavior)?;
        }
    }

    Ok(())
}

fn wire_behavior(&mut self, type_hash: TypeHash, func_hash: TypeHash, behavior: &Behavior) -> Result<(), ContextError> {
    // Get or create TypeBehaviors for this type
    let behaviors = self.registry.get_behaviors_mut(type_hash);

    match behavior {
        Behavior::Constructor => behaviors.add_constructor(func_hash),
        Behavior::CopyConstructor => behaviors.add_constructor(func_hash), // with flag?
        Behavior::Factory => behaviors.add_factory(func_hash),
        Behavior::Destructor => behaviors.set_destructor(func_hash),
        Behavior::AddRef => behaviors.set_addref(func_hash),
        Behavior::Release => behaviors.set_release(func_hash),
        Behavior::ListConstruct => behaviors.set_list_construct(func_hash),
        Behavior::ListFactory => behaviors.set_list_factory(func_hash),
        Behavior::TemplateCallback => {
            behaviors.template_callback = Some(func_hash);
            // Also register the callback function
            // This needs special handling - see Phase 4
        }
        Behavior::Operator(op) => {
            let op_behavior = op.to_operator_behavior();
            behaviors.add_operator(op_behavior, func_hash);
        }
        Behavior::GetWeakRefFlag => behaviors.get_weakref_flag = Some(func_hash),
        // GC behaviors...
        _ => {}
    }

    Ok(())
}
```

### Phase 4: Template Callback Handling

**Decision:** Template callbacks are just regular `NativeFn` functions with `Behavior::TemplateCallback`.

No separate storage needed - they go in the main `functions` map like any other behavior:
- Function registered with `Behavior::TemplateCallback`
- Function hash stored in `class_entry.behaviors.template_callback`
- Function stored in main `functions` map (not a separate `template_callbacks` map)

**Registry Cleanup:**
Remove `template_callbacks: FxHashMap<TypeHash, TemplateCallback>` from `SymbolRegistry`.
Remove `register_template_callback()`, `has_template_callback()`, `validate_template_instance()`.

**To call a template callback:**
1. Look up `class_entry.behaviors.template_callback` for the function hash
2. Look up function in main `functions` map
3. Call via `NativeFn` with `CallContext` containing template info

### Phase 5: Tests

#### 5.1 Requirement-Based Tests (Write First!)

```rust
#[test]
fn function_meta_captures_native_fn() {
    let meta = <add_numbers as HasFunctionMeta>::__as_fn_meta();
    assert!(meta.native_fn.is_some(), "macro must capture the callable");
}

#[test]
fn class_meta_captures_type_id() {
    let meta = Vec3::__as_type_meta();
    assert_eq!(meta.rust_type_id, Some(TypeId::of::<Vec3>()));
}

#[test]
fn native_fn_is_callable() {
    let meta = <add_numbers as HasFunctionMeta>::__as_fn_meta();
    let native = meta.native_fn.unwrap();

    // Set up CallContext with args
    let mut slots = vec![VmSlot::Int(10), VmSlot::Int(20)];
    let mut ret = VmSlot::Void;
    let mut heap = ObjectHeap::new();
    let mut ctx = CallContext::new(&mut slots, 0, &mut ret, &mut heap);

    native.call(&mut ctx).unwrap();
    assert_eq!(ret, VmSlot::Int(30));
}
```

#### 5.2 Integration Tests

```rust
#[test]
fn behaviors_registered_for_constructor() {
    let ctx = Context::new();
    let module = Module::new()
        .ty::<Player>()
        .function(Player::new__meta);
    ctx.install(module).unwrap();

    let player_hash = TypeHash::from_name("Player");
    let behaviors = ctx.registry().get_behaviors(player_hash).unwrap();
    assert!(behaviors.has_constructors());
}

#[test]
fn template_callback_registered() {
    let ctx = Context::new();
    let module = Module::new()
        .ty::<ScriptArray>()
        .function(ScriptArray::validate_template__meta);
    ctx.install(module).unwrap();

    let array_hash = TypeHash::from_name("array");
    let behaviors = ctx.registry().get_behaviors(array_hash).unwrap();
    // Template callback is stored as a behavior, looked up via function hash
    assert!(behaviors.template_callback.is_some());
}

#[test]
fn operator_behavior_registered() {
    let ctx = Context::new();
    let module = Module::new()
        .ty::<Vec3>()
        .function(Vec3::op_add__meta);
    ctx.install(module).unwrap();

    let vec3_hash = TypeHash::from_name("Vec3");
    let behaviors = ctx.registry().get_behaviors(vec3_hash).unwrap();
    assert!(behaviors.has_operator(OperatorBehavior::OpAdd));
}
```

## Dependencies

- `CallContext` needs `arg::<T>()` and `set_return::<T>()` methods (requires conversion traits)
- May need `FromScript`/`ToScript` traits or similar for type conversion
- Template callback signature needs to be compatible with storage

## Risks

1. **Macro complexity:** Generating correct wrapper code for all function signatures is complex
2. **Conversion traits:** Need trait bounds that the macro can rely on
3. **Generic functions:** Template functions with `?` types need special handling
4. **Performance:** Arc overhead for every function call (likely negligible)

## Phases Summary

| Phase | Description | Complexity |
|-------|-------------|------------|
| 1 | Core type changes | Low |
| 2 | Macro wrapper generation | High |
| 3 | Context behavior wiring | Medium |
| 4 | Template callback handling | Medium |
| 5 | Tests | Medium |

## Recommended Order

1. Write failing tests first (Phase 5.1)
2. Add fields to structs (Phase 1)
3. Update `install_function` for behaviors (Phase 3)
4. Tackle macro wrapper generation (Phase 2)
5. Handle template callbacks (Phase 4)
6. Integration tests (Phase 5.2)
