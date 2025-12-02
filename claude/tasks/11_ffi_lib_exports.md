# Task 11: Library Exports and Public API

**Status:** Not Started
**Depends On:** All previous tasks
**Estimated Scope:** Final integration

---

## Objective

Update `src/lib.rs` to export the new FFI API and organize the public interface.

## Files to Modify

- `src/lib.rs` - Add exports for FFI types

## Current Exports

```rust
pub use module::ScriptModule;
// ... other existing exports
```

## New Exports

```rust
// src/lib.rs

// Core compilation
pub use context::Context;
pub use unit::Unit;

// FFI Registration
pub mod ffi {
    pub use crate::ffi::module::Module;
    pub use crate::ffi::traits::{FromScript, ToScript, NativeType};
    pub use crate::ffi::types::{TypeSpec, RefModifier, TypeKind, ReferenceKind};
    pub use crate::ffi::native_fn::{NativeFn, CallContext};
    pub use crate::ffi::error::{NativeError, ModuleError, ContextError};
    pub use crate::ffi::any_type::{AnyRef, AnyRefMut};

    // Builders
    pub use crate::ffi::function::FunctionBuilder;
    pub use crate::ffi::class::{ClassBuilder, MethodBuilder, PropertyBuilder};
    pub use crate::ffi::enum_builder::EnumBuilder;
    pub use crate::ffi::interface::InterfaceBuilder;
    pub use crate::ffi::funcdef::FuncdefBuilder;
    pub use crate::ffi::template::{TemplateBuilder, TemplateInstanceBuilder};
}

// Built-in modules
pub mod modules {
    pub use crate::modules::{default_modules, std, string, array, dictionary, math};
}

// Legacy API (for backwards compatibility, can be deprecated later)
pub use module::ScriptModule;
```

## Prelude (Optional)

```rust
// src/lib.rs
pub mod prelude {
    pub use crate::Context;
    pub use crate::ffi::{Module, NativeType, FromScript, ToScript};
}
```

## Usage After

```rust
use angelscript::{Context, ffi::Module, modules};

fn main() {
    // Create context with default modules
    let ctx = Context::with_default_modules().unwrap();

    // Or build custom
    let mut ctx = Context::new();
    ctx.install(modules::string()).unwrap();
    ctx.install(modules::math()).unwrap();

    // Add custom module
    let mut game = Module::new(&["game"]);
    game.register_fn("spawn", |x: f32, y: f32| { /* ... */ });
    ctx.install(game).unwrap();

    // Compile scripts
    let mut unit = ctx.create_unit();
    unit.add_source("main.as", src).unwrap();
    unit.build().unwrap();
}
```

## Acceptance Criteria

- [ ] All FFI types accessible from `angelscript::ffi`
- [ ] All built-in modules accessible from `angelscript::modules`
- [ ] Context and Unit accessible at crate root
- [ ] Backwards compatibility with ScriptModule (if needed)
- [ ] Clean, documented public API
- [ ] Examples in crate-level documentation
