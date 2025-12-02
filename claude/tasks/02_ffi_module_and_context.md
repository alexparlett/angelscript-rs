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

## Design Decision: AST Primitive Reuse

Module stores FFI-specific container types that compose AST primitives. All parsed items use the existing AST types (`Ident`, `TypeExpr`, `FunctionParam`, `ReturnType`) stored in the Module's `Bump` arena.

## Key Types

```rust
/// A namespaced collection of native functions, types, and global properties.
///
/// The Module owns a Bump arena where parsed AST nodes are stored.
/// All FFI storage types use AST primitives with 'ast lifetime.
pub struct Module<'app> {
    namespace: Vec<String>,
    arena: Bump,  // Owns parsed AST nodes from declaration strings

    // FFI storage types using AST primitives (see ffi_plan.md "FFI Storage Types")
    functions: Vec<NativeFunctionDef<'_>>,      // Uses Ident, FunctionParam, ReturnType
    types: Vec<NativeTypeDef<'_>>,              // Uses Ident for template params
    enums: Vec<NativeEnumDef>,                   // Simple strings, no AST types needed
    interfaces: Vec<NativeInterfaceDef<'_>>,    // Uses Ident, FunctionParam, ReturnType
    funcdefs: Vec<NativeFuncdefDef<'_>>,        // Uses Ident, FunctionParam, ReturnType
    global_properties: Vec<GlobalPropertyDef<'app, '_>>,  // Uses Ident, TypeExpr
}

impl<'app> Module<'app> {
    pub fn new(namespace: &[&str]) -> Self;
    pub fn root() -> Self;
    pub fn register_fn<F, Args, Ret>(&mut self, decl: &str, f: F) -> Result<&mut Self, FfiRegistrationError>;
    pub fn register_fn_raw<F>(&mut self, decl: &str, f: F) -> Result<&mut Self, FfiRegistrationError>;
    pub fn register_global_property<T: 'static>(&mut self, decl: &str, value: &'app mut T) -> Result<&mut Self, FfiRegistrationError>;
    pub fn register_type<T: NativeType>(&mut self, name: &str) -> ClassBuilder<'_, 'app, T>;
    pub fn register_enum(&mut self, name: &str) -> EnumBuilder<'_, 'app>;
    pub fn register_interface(&mut self, name: &str) -> InterfaceBuilder<'_, 'app>;
    pub fn register_funcdef(&mut self, decl: &str) -> Result<&mut Self, FfiRegistrationError>;
}

/// The scripting context. Install modules and create compilation units.
pub struct Context {
    modules: Vec<Module<'static>>,
}

impl Context {
    pub fn new() -> Self;
    pub fn with_default_modules() -> Result<Self, ContextError>;
    pub fn install(&mut self, module: Module<'static>) -> Result<(), ContextError>;
    pub fn create_unit(&self) -> Unit;
}
```

## FFI Storage Types Reference

These types are defined in `ffi_plan.md` and use AST primitives. IDs are assigned at registration time using the global atomic counters (`TypeId::next()` and `FunctionId::next()`).

```rust
// Functions use AST primitives for signature
pub struct NativeFunctionDef<'ast> {
    pub id: FunctionId,  // Assigned at registration via FunctionId::next()
    pub name: Ident<'ast>,
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
    pub is_const: bool,
    pub native_fn: NativeFn,
}

// Enums don't need AST types (builder provides resolved values)
pub struct NativeEnumDef {
    pub id: TypeId,  // Assigned at registration via TypeId::next()
    pub name: String,
    pub values: Vec<(String, i64)>,
}

// Interfaces use AST primitives for method signatures
pub struct NativeInterfaceDef<'ast> {
    pub id: TypeId,  // Assigned at registration via TypeId::next()
    pub name: String,
    pub methods: Vec<NativeInterfaceMethod<'ast>>,
}

// Interface methods are abstract signatures - NO FunctionId
pub struct NativeInterfaceMethod<'ast> {
    pub name: Ident<'ast>,
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
    pub is_const: bool,
}

// Funcdefs use AST primitives
pub struct NativeFuncdefDef<'ast> {
    pub id: TypeId,  // Assigned at registration via TypeId::next()
    pub name: Ident<'ast>,
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
}

// Global properties use AST primitives for type info
pub struct GlobalPropertyDef<'app, 'ast> {
    pub name: Ident<'ast>,
    pub type_expr: &'ast TypeExpr<'ast>,
    pub is_const: bool,
    pub value: GlobalPropertyRef<'app>,
}
```

## Implementation Notes

- Module has `'app` lifetime for global property references
- Module owns a `Bump` arena for parsed AST nodes from declaration strings
- All parsed data uses existing AST types: `Ident`, `TypeExpr`, `FunctionParam`, `ReturnType`
- Context owns installed modules
- Namespaces are immutable per-module
- `register_fn` takes declaration string (e.g., `"float sqrt(float x)"`)
- `register_enum` and `register_interface` return builders
- `register_funcdef` takes full declaration string (e.g., `"funcdef void Callback()"`)
- Templates are registered via `register_type` with `<class T>` syntax

## Acceptance Criteria

- [ ] Module can be created with namespaces
- [ ] Context can install modules
- [ ] Global properties can be registered with lifetime tracking
- [ ] Module owns Bump arena for parsed AST nodes
- [ ] Basic tests for module creation and installation
