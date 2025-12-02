# Task 02: Module and Context API

**Status:** Not Started
**Depends On:** Task 01
**Estimated Scope:** Public API layer

---

## Objective

Implement the Module and Context types that form the public API for FFI registration.

## Files to Create

- `src/ffi/module.rs` - Module struct with registration methods
- `src/ffi/context.rs` - Context struct that owns modules
- `src/ffi/global_property.rs` - GlobalPropertyDef, GlobalPropertyRef

## Key Types

```rust
/// A namespaced collection of native functions, types, and global properties.
pub struct Module<'app> {
    namespace: Vec<String>,
    functions: Vec<FunctionDef>,
    types: Vec<TypeDef>,
    enums: Vec<EnumDef>,
    templates: Vec<TemplateDef>,
    global_properties: Vec<GlobalPropertyDef<'app>>,
}

impl<'app> Module<'app> {
    pub fn new(namespace: &[&str]) -> Self;
    pub fn root() -> Self;
    pub fn register_fn<F, Args, Ret>(&mut self, name: &str, f: F) -> &mut Self;
    pub fn register_global_property<T: NativeType>(&mut self, decl: &str, value: &'app mut T) -> Result<(), ModuleError>;
    pub fn register_type<T: NativeType>(&mut self, name: &str) -> ClassBuilder<'_, T>;
    pub fn register_enum(&mut self, name: &str) -> EnumBuilder<'_>;
    pub fn register_interface(&mut self, name: &str) -> InterfaceBuilder<'_>;
    pub fn register_funcdef(&mut self, name: &str) -> FuncdefBuilder<'_>;
    pub fn register_template(&mut self, name: &str) -> TemplateBuilder<'_>;
}

/// The scripting context. Install modules and create compilation units.
pub struct Context {
    modules: Vec<Module>,
}

impl Context {
    pub fn new() -> Self;
    pub fn with_default_modules() -> Result<Self, ContextError>;
    pub fn install(&mut self, module: Module) -> Result<(), ContextError>;
    pub fn create_unit(&self) -> Unit;
}
```

## Implementation Notes

- Module has `'app` lifetime for global property references
- Context owns installed modules
- Namespaces are immutable per-module
- `register_fn` should infer signature from closure types

## Acceptance Criteria

- [ ] Module can be created with namespaces
- [ ] Context can install modules
- [ ] Global properties can be registered with lifetime tracking
- [ ] Basic tests for module creation and installation
