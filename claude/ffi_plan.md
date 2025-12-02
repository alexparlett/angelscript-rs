# FFI Registration System Plan

**Status:** Ready for Review
**Date:** 2025-12-02

## Overview

Design and implement a comprehensive FFI registration API for angelscript-rust that allows users to register native Rust functions and types. This replaces the current hardcoded built-in implementations and provides a complete API for:

- Global functions and methods
- Classes (value types and reference types)
- Enums
- Interfaces
- Funcdefs (function pointers)
- Templates with validation callbacks
- Variable parameter types (`?&`)

**Scope:** Registration API only - populating the Registry for semantic analysis. Not VM execution.

---

## Research Summary

### AngelScript C++

**Types:**
- **Reference types**: Dynamic memory, support handles, cannot pass by value
- **Value types**: Stack/member, no handles, can pass by value
- Object behaviors: constructors, destructors, factories, addref/release

**Templates:**
- Registered with `asOBJ_TEMPLATE` flag
- Factory/constructor receives hidden `asITypeInfo*` parameter for subtype info
- `asBEHAVE_TEMPLATE_CALLBACK` validates instantiations at compile time
- Callback can disable GC for specific instantiations
- Subtypes can only be passed by reference/handle (not by value)
- Specializations override generic template for specific types

**Variable Parameter Type (`?&`):**
- Reference to any type
- Function receives both `void* ref` and `int typeId`
- Works with `&in` and `&out`, not `&inout`
- Only for global functions, constructors, methods (not operators)

**Generic Calling Convention:**
- Portable fallback when native conventions unsupported
- Function signature: `void fn(asIScriptGeneric* gen)`
- Manually extract args via `GetArgDWord()`, `GetArgObject()`, etc.
- Return via `SetReturnObject()`, `SetReturnAddress()`

### Rhai
- `Engine::register_fn` with automatic type conversion
- `TypeBuilder` pattern for methods/properties
- `Dynamic` type as universal runtime value
- Low-level `register_raw_fn` for direct Dynamic access

### Rune
- `Module` as collection of functions/types
- `#[function]` attribute macro
- `ToValue`/`FromValue` traits for conversion

---

## Architecture

### Design Principles

1. **Builder pattern initially**, macros later for convenience
2. **Function pointers stored for semantic analysis** - not just signatures
3. **Template callback pattern** inspired by AngelScript
4. **Full coverage**: enums, interfaces, funcdefs, templates, variadics
5. **Shareable across modules** - FFI definitions can be reused

### Top-Level API: Context + Module

Inspired by Rune's module system, native registrations are organized into **Module**s that each have a namespace. Modules are installed into a `Context`, and scripts access items via their namespace.

**Module** - A namespaced collection of native functions/types:
```rust
// Create a module with a namespace
let mut math = Module::new(&["math"]);
math.register_fn("sqrt", |x: f64| x.sqrt());
math.register_fn("sin", |x: f64| x.sin());
math.register_type::<Vec3>("Vec3")
    .value_type()
    .method("length", Vec3::length)
    .build();

// Nested namespaces using array syntax
let mut collections = Module::new(&["std", "collections"]);
collections.register_template("HashMap")...;
// In script: std::collections::HashMap<string, int>

// More nesting examples
let mut game_physics = Module::new(&["game", "physics"]);
game_physics.register_type::<RigidBody>("RigidBody")
    .reference_type()
    .build();
// In script: game::physics::RigidBody@ body;
```

**Context** - Owns installed native modules, creates compilation units:
```rust
// Option 1: Load all default modules (string, array, dictionary, math)
let mut ctx = Context::with_default_modules()?;

// Option 2: Start empty and selectively install built-ins
let mut ctx = Context::new();
ctx.install(angelscript::modules::string())?;
ctx.install(angelscript::modules::array())?;
// Skip dictionary if not needed

// Option 3: Custom modules only
let mut ctx = Context::new();
ctx.install(math)?;
ctx.install(game_physics)?;

// Create compilation units from the context
let mut unit = ctx.create_unit();
unit.add_source("game.as", src)?;
unit.build()?;
```

**Script access via namespace:**
```angelscript
// Access registered items via namespace
void main() {
    float x = math::sqrt(16.0);
    math::Vec3 v(1, 2, 3);
    float len = v.length();

    game::physics::RigidBody@ body = createBody();
}

// Or import the namespace
using namespace math;
void main() {
    float x = sqrt(16.0);  // Now accessible without prefix
    Vec3 v(1, 2, 3);
}
```

**Nested namespaces:**
```rust
// Array syntax creates nested namespace hierarchy:
//   std (namespace)
//     └── collections (namespace)
//           └── HashMap (type)
let mut collections = Module::new(&["std", "collections"]);
collections.register_template("HashMap")...;

// Single-level namespace:
let mut math = Module::new(&["math"]);

// In script - full path required:
std::collections::HashMap<string, int> map;

// Or import the parent namespace:
using namespace std;
collections::HashMap<string, int> map;

// Or import directly:
using namespace std::collections;
HashMap<string, int> map;
```

**Root namespace for globals:**
```rust
// Root namespace (no prefix needed in script)
let mut globals = Module::root();
globals.register_fn("print", |s: &str| println!("{}", s));
ctx.install(globals)?;

// In script: print("hello") - no namespace prefix
```

### Internal Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  Module (public, namespaced collection)                         │
│  ├── namespace: Vec<String>  (empty = root, ["math"], ["std", "collections"])  │
│  ├── functions: Vec<FunctionDef>                            │
│  ├── types: Vec<TypeDef>                                    │
│  ├── enums: Vec<EnumDef>                                    │
│  └── templates: Vec<TemplateDef>                            │
│                                                             │
│  Methods:                                                   │
│  ├── new(namespace) → Module                                │
│  ├── register_fn() → FunctionBuilder                        │
│  ├── register_type<T>() → ClassBuilder                      │
│  ├── register_enum() → EnumBuilder                          │
│  └── register_template() → TemplateBuilder                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ install()
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Context                                                    │
│  ├── modules: Vec<Module>  (installed modules)              │
│  │                                                          │
│  │  Methods:                                                │
│  │  ├── new() → Context  (with built-ins installed)         │
│  │  ├── new_raw() → Context  (no built-ins)                 │
│  │  ├── install(module) → Result<()>                        │
│  │  └── create_unit() → Unit                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ create_unit()
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Unit (compilation unit)                                    │
│  ├── context: Arc<Context>  (shared reference)              │
│  ├── sources: HashMap<String, String>                       │
│  ├── arena: Bump                                            │
│  └── build() applies all Modules to Registry                │
└─────────────────────────────────────────────────────────────┘
```

**Why This Design?**
1. `Module` is the unit of native registration - each has a namespace
2. `Context` collects installed modules and creates compilation units
3. Multiple units share the same context's native registrations
4. Namespaces are immutable per-module (no stateful `SetDefaultNamespace`)
5. Built-ins (`string`, `array`, `dictionary`) are pre-installed modules in root namespace

### Value vs Reference Types: Scope Boundary

This FFI plan covers **registration** - storing type metadata and function pointers for semantic analysis. The VM execution layer (calling behaviors, memory management) is separate.

**What the FFI registers (this plan):**
- Type kind (value vs reference)
- Size/alignment for value types
- Behavior function pointers (factory, addref, release, construct, destruct)
- Methods, operators, properties

**What the VM executes (not this plan):**
- Calling factory to create reference type instances
- Calling addref/release for handle operations
- Allocating stack space for value types
- Calling construct/destruct at appropriate times

**Handles (`@`):**
- Script code uses `Foo@` to create handles to reference types
- Semantic analysis validates handle usage based on registered type kind
- VM manages the actual reference counting

**Auto Handles (`@+`):**
- Declared with `@+` suffix in parameter/return types
- VM automatically calls AddRef for incoming parameters and Release after use
- For return values, VM automatically calls AddRef before returning
- Reduces boilerplate in native functions - no manual ref count management
- Only applicable to reference types with ref counting (not Scoped or SingleRef)

**Reference Type Variants:**
- **Standard** - Full handle support with AddRef/Release ref counting
- **Scoped** - RAII-style, destroyed at scope exit, no handles (asOBJ_SCOPED)
- **SingleRef** - App-controlled lifetime, no handles in script (asOBJ_NOHANDLE)
- **GenericHandle** - Type-erased container that can hold any type (asOBJ_ASHANDLE)

### VM Storage Design (Pointer-Free)

The VM stores all values without raw pointers, using safe Rust constructs:

**Design Principles:**
1. **No raw pointers** - All object access via `Box<dyn Any>` with `downcast_ref`/`downcast_mut`
2. **Generational handles** - `ObjectHandle` with index + generation prevents use-after-free
3. **TypeId verification** - Every cast is validated at runtime via `TypeId`
4. **Enum-based stack values** - Primitives stored inline in `VmSlot` enum

**Storage Strategy:**
- **Primitives** (int, float, bool): Stored directly in `VmSlot` enum variants
- **Strings**: Owned `String` in `VmSlot::String`
- **Small value types** (≤ size threshold): Stored inline as `VmSlot::Native(Box<dyn Any>)`
- **Reference types**: Heap-allocated via `ObjectHeap`, accessed through `ObjectHandle`

**How FFI Uses This:**
- `CallContext` holds `&mut ObjectHeap` for object access
- `ctx.this::<T>()` uses `downcast_ref::<T>()` - safe, returns `Option`
- `FromScript`/`ToScript` traits convert between `VmSlot` and Rust types
- `AnyRef` wraps the slot variant for `?&` parameters - no pointer casting

**Trade-offs:**
- **Pro**: Completely memory-safe, no undefined behavior possible
- **Pro**: Generational handles catch use-after-free at runtime
- **Con**: Runtime type checks on every access (small overhead)
- **Con**: `Box<dyn Any>` requires heap allocation for native types

### Calling Convention Design

Native functions need to bridge between Rust and the VM. We support **two calling conventions**, similar to AngelScript C++ which has both native and generic conventions:

| Convention | API | Use Case | Signature Declaration |
|------------|-----|----------|----------------------|
| **Type-Safe** | `register_fn`, `method` | Simple functions, known types | Inferred from Rust closure |
| **Generic** | `register_fn_raw`, `method_raw` | `?&` params, complex logic | Explicit via `FunctionBuilder` |

**Why both?**
- Type-safe is ergonomic for 90% of cases - just pass a closure
- Generic is required for `?&` (variable type) parameters where type isn't known at registration
- Generic gives full control for performance-critical code or complex conditional logic

Both conventions store a `NativeFn` internally - the difference is in how arguments are extracted:

**1. Type-Safe (High-Level)** - Idiomatic Rust signatures with automatic conversion:
```rust
// Global function - direct Rust types
module.register_fn("sqrt", |x: f64| x.sqrt());
module.register_fn("contains", |s: &ScriptString, needle: &str| -> bool {
    s.as_str().contains(needle)
});

// Method - self is first parameter, inferred from ClassBuilder<T>
module.register_type::<Vec3>("Vec3")
    .method("length", |this: &Vec3| this.length())           // &self
    .method("normalize", |this: &mut Vec3| this.normalize()) // &mut self
    .method("add", |this: &Vec3, other: &Vec3| *this + *other); // &self + params
```

**2. Generic (Low-Level)** - Manual argument extraction for complex cases:
```rust
// For ?& parameters, complex types, or full control
module.register_fn_raw("format", |ctx: &mut CallContext| -> Result<(), NativeError> {
    let fmt: &str = ctx.arg::<&str>(0)?;
    let any_val = ctx.arg_any(1)?;  // ?&in - returns AnyRef
    let type_id = any_val.type_id();
    // ... format based on type
    ctx.set_return(result);
    Ok(())
});

// Methods with raw context
module.register_type::<Foo>("Foo")
    .method_raw("complex", |ctx: &mut CallContext| {
        let this: &Foo = ctx.this()?;  // Get self reference
        let arg = ctx.arg_any(0)?;      // ?&in parameter
        // ...
    })
    .with_signature("void complex(?&in)");  // Explicit signature required
```

**Signature Declaration:**
- **Type-safe**: Signature inferred from Rust closure types via `FromScript`/`ToScript`
- **Generic**: Signature declared explicitly via `FunctionBuilder::with_signature()` or builder methods:
  ```rust
  module.register_fn_raw("format", |ctx| { ... })
      .param::<&str>("fmt")       // Typed param
      .param_any_in("value")      // ?&in param
      .returns::<String>();       // Return type
  ```

**Self Handling for Methods:**

For type-safe methods, `self` is always the first parameter in the closure:
```rust
// These are equivalent:
.method("length", |this: &Vec3| this.length())
.method("length", Vec3::length)  // fn(&self) -> f32

// The ClassBuilder knows T, so it can:
// 1. Extract `this` from VM's first argument slot
// 2. Cast it to &T or &mut T based on closure signature
// 3. Pass remaining VM args to the closure's other parameters
```

For raw methods, use `ctx.this::<T>()`:
```rust
.method_raw("foo", |ctx: &mut CallContext| {
    let this: &Foo = ctx.this()?;        // Immutable borrow
    let this: &mut Foo = ctx.this_mut()?; // Mutable borrow
});
```

### Core Traits

```rust
// src/ffi/traits.rs

/// Maps Rust types to AngelScript DataType for parameters
pub trait FromScript: Sized {
    /// The AngelScript type(s) this can convert from
    fn script_type() -> DataType;
}

/// Maps Rust types to AngelScript DataType for return values
pub trait ToScript {
    /// The AngelScript type this produces
    fn script_type() -> DataType;
}

/// Marker for types that can be registered as native types
pub trait NativeType: 'static {
    /// Type name in AngelScript
    const NAME: &'static str;
}
```

### Native Function Storage

Functions are stored type-erased but called through a generic context:

```rust
// src/ffi/native_fn.rs

/// Type-erased native function
pub struct NativeFn {
    inner: Box<dyn NativeCallable + Send + Sync>,
}

/// Trait for callable native functions
pub trait NativeCallable {
    fn call(&self, ctx: &mut CallContext) -> Result<(), NativeError>;
}

/// Context for native function calls - bridges VM and Rust
pub struct CallContext<'vm> {
    /// VM stack/argument slots
    slots: &'vm mut [VmSlot],
    /// Index of first argument (0 for functions, 1 for methods where 0 is `this`)
    arg_offset: usize,
    /// Return value slot
    return_slot: &'vm mut VmSlot,
    /// Object heap for reference type access
    heap: &'vm mut ObjectHeap,
    /// Type registry for runtime type info
    registry: &'vm Registry,
}

impl<'vm> CallContext<'vm> {
    /// Get typed argument at index (for type-safe wrappers)
    pub fn arg<T: FromScript>(&self, index: usize) -> Result<T, NativeError>;

    /// Get `this` reference for methods (immutable)
    /// Works with both VmSlot::Object (heap) and VmSlot::Native (inline)
    pub fn this<T: NativeType>(&self) -> Result<&T, NativeError> {
        match &self.slots[0] {
            VmSlot::Object(handle) => {
                self.heap.get::<T>(*handle)
                    .ok_or(NativeError::TypeMismatch)
            }
            VmSlot::Native(boxed) => {
                boxed.downcast_ref::<T>()
                    .ok_or(NativeError::TypeMismatch)
            }
            _ => Err(NativeError::InvalidThis),
        }
    }

    /// Get `this` reference for methods (mutable)
    pub fn this_mut<T: NativeType>(&mut self) -> Result<&mut T, NativeError> {
        match &mut self.slots[0] {
            VmSlot::Object(handle) => {
                let handle = *handle;  // Copy to avoid borrow conflict
                self.heap.get_mut::<T>(handle)
                    .ok_or(NativeError::TypeMismatch)
            }
            VmSlot::Native(boxed) => {
                boxed.downcast_mut::<T>()
                    .ok_or(NativeError::TypeMismatch)
            }
            _ => Err(NativeError::InvalidThis),
        }
    }

    /// Get any-typed argument (?&in parameters)
    pub fn arg_any(&self, index: usize) -> Result<AnyRef<'_>, NativeError>;
    pub fn arg_any_mut(&mut self, index: usize) -> Result<AnyRefMut<'_>, NativeError>;

    /// Set return value
    pub fn set_return<T: ToScript>(&mut self, value: T) -> Result<(), NativeError>;

    /// Get type info for runtime checks
    pub fn type_info(&self, type_id: TypeId) -> Option<&TypeDef>;
}

/// A slot in the VM that holds a value (no raw pointers)
#[derive(Clone)]
pub enum VmSlot {
    /// Void/empty
    Void,
    /// Primitive integer (i8, i16, i32, i64, u8, u16, u32, u64)
    Int(i64),
    /// Floating point (f32, f64)
    Float(f64),
    /// Boolean
    Bool(bool),
    /// String value (owned)
    String(String),
    /// Handle to heap-allocated object (reference types)
    Object(ObjectHandle),
    /// Inline native value (small registered types stored directly)
    /// Uses Box<dyn Any> for type safety - no raw pointer casting
    Native(Box<dyn Any + Send + Sync>),
    /// Null handle
    NullHandle,
}

/// Handle to a heap-allocated object - safe, copyable reference
#[derive(Clone, Copy, Debug)]
pub struct ObjectHandle {
    /// Index into ObjectHeap.slots
    pub index: u32,
    /// Generation for use-after-free detection
    pub generation: u32,
    /// Type for runtime verification
    pub type_id: TypeId,
}

/// Heap storage for reference types with generational indices
pub struct ObjectHeap {
    slots: Vec<HeapSlot>,
    free_list: Vec<u32>,
}

struct HeapSlot {
    generation: u32,
    value: Option<Box<dyn Any + Send + Sync>>,
    ref_count: u32,  // For reference-counted types
}

impl ObjectHeap {
    /// Allocate a new object on the heap
    pub fn allocate<T: Any + Send + Sync>(&mut self, value: T) -> ObjectHandle;

    /// Get immutable reference (returns None if stale handle or wrong type)
    pub fn get<T: Any>(&self, handle: ObjectHandle) -> Option<&T>;

    /// Get mutable reference
    pub fn get_mut<T: Any>(&mut self, handle: ObjectHandle) -> Option<&mut T>;

    /// Increment reference count
    pub fn add_ref(&mut self, handle: ObjectHandle);

    /// Decrement reference count, free if zero
    pub fn release(&mut self, handle: ObjectHandle) -> bool;

    /// Free object (for scoped types)
    pub fn free(&mut self, handle: ObjectHandle);
}
```

### Type Mappings

| Rust Type | AngelScript Type | RefModifier |
|-----------|------------------|-------------|
| `()` | `void` | - |
| `bool` | `bool` | - |
| `i8/i16/i32/i64` | `int8/int16/int/int64` | - |
| `u8/u16/u32/u64` | `uint8/uint16/uint/uint64` | - |
| `f32/f64` | `float/double` | - |
| `String` | `string` | - |
| `&str` | `const string &in` | In |
| `&T` | `const T &in` | In |
| `&mut T` | `T &inout` | InOut |
| `Out<T>` wrapper | `T &out` | Out |

### Variable Parameter Type

For `?&` (any type) parameters - uses enum-based storage, no raw pointers:

```rust
/// Type-erased reference for ?&in parameters
pub enum AnyRef<'a> {
    /// Primitive value (copied)
    Int(i64),
    Float(f64),
    Bool(bool),
    /// String reference
    String(&'a str),
    /// Reference to heap object
    Object { handle: ObjectHandle, heap: &'a ObjectHeap },
    /// Reference to inline native value
    Native(&'a dyn Any),
}

/// Type-erased mutable reference for ?&out parameters
pub enum AnyRefMut<'a> {
    /// Mutable primitives via wrapper
    Int(&'a mut i64),
    Float(&'a mut f64),
    Bool(&'a mut bool),
    /// Mutable string
    String(&'a mut String),
    /// Mutable reference to heap object
    Object { handle: ObjectHandle, heap: &'a mut ObjectHeap },
    /// Mutable reference to inline native value
    Native(&'a mut dyn Any),
}

impl<'a> AnyRef<'a> {
    /// Get the TypeId of the contained value
    pub fn type_id(&self) -> TypeId;

    /// Try to downcast to a concrete type
    pub fn downcast<T: Any>(&self) -> Option<&T> {
        match self {
            AnyRef::Object { handle, heap } => heap.get::<T>(*handle),
            AnyRef::Native(any) => any.downcast_ref::<T>(),
            _ => None,  // Primitives handled separately
        }
    }

    /// Check if this is a specific primitive type
    pub fn is_int(&self) -> bool;
    pub fn is_float(&self) -> bool;
    pub fn is_bool(&self) -> bool;
    pub fn is_string(&self) -> bool;

    /// Get primitive values
    pub fn as_int(&self) -> Option<i64>;
    pub fn as_float(&self) -> Option<f64>;
    pub fn as_bool(&self) -> Option<bool>;
    pub fn as_str(&self) -> Option<&str>;
}
```

---

## Public API

### Module

A namespaced collection of native functions, types, and global properties. This is the public entry point for registering native items.

```rust
// src/module.rs

/// A namespaced collection of native functions, types, and global properties.
///
/// The `'app` lifetime parameter ensures global property references outlive the module.
pub struct Module<'app> {
    /// Namespace path for all items. Empty = root namespace, ["math"] = single level,
    /// ["std", "collections"] = nested namespace (std -> collections)
    namespace: Vec<String>,
    /// Registered functions
    functions: Vec<FunctionDef>,
    /// Registered types
    types: Vec<TypeDef>,
    /// Registered enums
    enums: Vec<EnumDef>,
    /// Registered templates
    templates: Vec<TemplateDef>,
    /// Global properties (app-owned references)
    global_properties: Vec<GlobalPropertyDef<'app>>,
}

impl<'app> Module<'app> {
    /// Create a new module with the given namespace path.
    /// Examples: &["math"], &["std", "collections"]
    pub fn new(namespace: &[&str]) -> Self;

    /// Create a module in the root namespace (no prefix needed in scripts).
    /// Equivalent to `Module::new(&[])`.
    pub fn root() -> Self;

    /// Register a global native function.
    pub fn register_fn<F, Args, Ret>(&mut self, name: &str, f: F) -> &mut Self
    where
        F: IntoNativeFn<Args, Ret>;

    /// Register a global property. The app owns the data; scripts read/write via reference.
    pub fn register_global_property<T: NativeType>(
        &mut self,
        decl: &str,
        value: &'app mut T,
    ) -> Result<(), ModuleError>;

    /// Register a native class type.
    pub fn register_type<T: NativeType>(&mut self, name: &str) -> ClassBuilder<'_, T>;

    /// Register a native enum.
    pub fn register_enum(&mut self, name: &str) -> EnumBuilder<'_>;

    /// Register a native interface.
    pub fn register_interface(&mut self, name: &str) -> InterfaceBuilder<'_>;

    /// Register a funcdef (function pointer type).
    pub fn register_funcdef(&mut self, name: &str) -> FuncdefBuilder<'_>;

    /// Register a template type.
    pub fn register_template(&mut self, name: &str) -> TemplateBuilder<'_>;
}
```

### Context

Owns installed modules and creates compilation units.

```rust
// src/context.rs

/// The scripting context. Install modules and create compilation units.
pub struct Context {
    /// Installed native modules
    modules: Vec<Module>,
}

impl Context {
    /// Create an empty context (no built-ins installed).
    pub fn new() -> Self;

    /// Create a context with all default modules installed.
    /// Equivalent to calling `new()` then installing string, array, dictionary, math.
    pub fn with_default_modules() -> Result<Self, ContextError>;

    /// Install a native module into this context.
    pub fn install(&mut self, module: Module) -> Result<(), ContextError>;

    /// Create a new compilation unit from this context.
    pub fn create_unit(&self) -> Unit;
}
```

### Global Properties

Global properties allow scripts to read and write app-owned data. Following the AngelScript C++ pattern, all globals are passed by reference - the app owns the data, and the engine stores a reference to it.

**Design Decision:** Global properties are registered on **Module** (not Context), following the same pattern as functions and types. The `'app` lifetime on Module ensures references outlive script execution.

**Internal Storage:**
```rust
// src/ffi/global_property.rs

/// Internal storage for a global property reference
pub struct GlobalPropertyDef<'app> {
    pub name: String,
    pub type_spec: TypeSpec,
    pub is_const: bool,
    pub value: GlobalPropertyRef<'app>,
}

/// Type-erased reference to global property value
pub enum GlobalPropertyRef<'app> {
    Int(&'app mut i64),
    Float(&'app mut f64),
    Bool(&'app mut bool),
    String(&'app mut String),
    Native { ptr: *mut (), type_id: std::any::TypeId },
    Handle(&'app mut Option<ObjectHandle>),
}
```

**Usage:**
```rust
// Global properties go in a module with their namespace
let mut g_score: i32 = 0;
let mut pi = std::f64::consts::PI;

// Root namespace - accessible without prefix
let mut globals = Module::root();
globals.register_global_property("int g_score", &mut g_score)?;

// Namespaced - accessible as math::PI
let mut math = Module::new(&["math"]);
math.register_global_property("const float PI", &mut pi)?;

ctx.install(globals)?;
ctx.install(math)?;
```

**Script Access:**
```angelscript
void main() {
    g_score = 100;              // Root namespace
    float r = math::PI * 2.0;   // Namespaced
}
```

**Key Points:**
1. Global properties are registered on **Module**, following the same pattern as functions/types
2. Each module has its own namespace - globals inherit that namespace
3. App owns all global data - changes on Rust side are visible to script and vice versa
4. `const` in declaration makes it read-only from script's perspective
5. Lifetime `'app` on Module ensures references remain valid during script execution

### Built-in Modules

Each built-in type is its own module, allowing selective installation:

```rust
// src/native/builtins/mod.rs

/// Returns all default modules
pub fn default_modules() -> Vec<Module> {
    vec![
        std(),
        string(),
        array(),
        dictionary(),
        math(),
    ]
}

/// Standard library module (in root namespace)
/// Contains: print, println, eprint, eprintln
pub fn std() -> Module { ... }

/// String type module (in root namespace)
pub fn string() -> Module { ... }

/// Array template module (in root namespace)
pub fn array() -> Module { ... }

/// Dictionary template module (in root namespace)
pub fn dictionary() -> Module { ... }

/// Math functions module (in "math" namespace)
pub fn math() -> Module { ... }
```

Usage:
```rust
use angelscript::modules;

// Install only what you need
let mut ctx = Context::new();
ctx.install(modules::string())?;
ctx.install(modules::array())?;
// math::sin(), math::cos() available in scripts
ctx.install(modules::math())?;
```

**Cargo Features (future consideration):**
Dangerous modules (like IO, filesystem, network) can be gated behind cargo features:
```toml
[features]
default = ["string", "array", "dictionary", "math"]
io = []  # File I/O operations
net = [] # Network operations
all = ["io", "net"]
```

```rust
// Only available with `io` feature
#[cfg(feature = "io")]
pub fn io() -> Module { ... }
```

### Unit (Compilation Unit)

```rust
// src/unit.rs

/// A compilation unit - compiles AngelScript source code.
pub struct Unit {
    /// Reference to the context's native modules
    context: Arc<Context>,
    /// Source files to compile
    sources: HashMap<String, String>,
    /// Memory arena (created during build)
    arena: Option<Bump>,
    /// Compiled module
    compiled: Option<CompiledUnit>,
    is_built: bool,
}

impl Unit {
    /// Note: Use `context.create_unit()` instead of `Unit::new()`
    pub(crate) fn new(context: Arc<Context>) -> Self;

    /// Add source code to compile.
    pub fn add_source(&mut self, name: &str, source: &str) -> Result<(), UnitError>;

    /// Build the unit (parse, analyze, compile).
    /// Native modules from the context are applied to the Registry during this step.
    pub fn build(&mut self) -> Result<(), BuildError>;
}
```

**Performance:** After native modules are applied to Registry:
- Single lookup path for all type/function resolution
- No performance penalty for native vs script-defined items
- Native functions stored in same `functions: HashMap<FunctionId, FunctionDef>`
- Native types stored in same `types: Vec<TypeDef>` with same `TypeId` allocation

### Semantic Analysis Compatibility

Native module registrations must provide complete information for compile-time checking. When `apply_to_registry()` runs, it creates full `FunctionDef` and `TypeDef` entries.

**FunctionDef requirements** (for type checking, overload resolution, const correctness):
```rust
pub struct FunctionDef<'src, 'ast> {
    pub id: FunctionId,
    pub name: String,
    pub params: Vec<DataType>,           // Full DataType with is_const, ref_modifier
    pub return_type: DataType,
    pub object_type: Option<TypeId>,     // For methods
    pub traits: FunctionTraits,          // is_const, is_constructor, etc.
    pub is_native: bool,                 // Set to true for native
    pub default_args: Vec<Option<&'ast Expr>>,  // Parsed from string during apply
    pub visibility: Visibility,
}
```

**Internal storage** (lifetime-free, converted to FunctionDef during apply):
```rust
pub struct NativeFunctionDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: TypeSpec,
    pub object_type: Option<String>,     // Type name, resolved to TypeId during apply
    pub traits: FunctionTraits,
    pub default_exprs: Vec<Option<String>>,  // Parsed during apply
    pub visibility: Visibility,
    pub native_fn: NativeFn,
}

pub struct ParamDef {
    pub name: String,
    pub type_spec: TypeSpec,
}

/// AngelScript type specification - stored explicitly, NOT inferred from Rust types.
/// This allows declaring signatures like `int@` (handle to primitive) that have
/// no Rust equivalent, or `const Foo@` vs `Foo @const` distinctions.
pub struct TypeSpec {
    pub type_name: String,               // Resolved to TypeId during apply
    pub is_const: bool,                  // `const T` - the value is const
    pub is_handle: bool,                 // `T@` - this is a handle
    pub is_handle_to_const: bool,        // `const T@` - handle points to const
    pub ref_modifier: RefModifier,       // &in, &out, &inout
}
```

**What semantic analysis checks (all work with native functions):**
- Type compatibility for function arguments
- Const correctness (can't call non-const method on const object)
- Overload resolution with type coercion
- Default argument evaluation
- Handle vs value type semantics
- Reference modifier compatibility (&in, &out, &inout)

### Function Builder

```rust
// src/native/function.rs

pub struct FunctionBuilder<'m> {
    module: &'m mut Module,
    name: String,
    params: Vec<ParamDef>,
    return_type: Option<TypeSpec>,
    native_fn: Option<NativeFn>,
    default_exprs: Vec<Option<String>>,  // Parsed into arena during apply
}

impl<'m> FunctionBuilder<'m> {
    /// Declare full signature from string (alternative to individual param/returns calls)
    /// e.g., "string format(const string &in fmt, ?&in value)"
    pub fn with_signature(mut self, sig: &str) -> Self;

    /// Add a parameter (type inferred from Rust type)
    pub fn param<T: FromScript>(mut self, name: &str) -> Self;

    /// Add a parameter with explicit reference modifier
    pub fn param_ref<T: FromScript>(mut self, name: &str, modifier: RefModifier) -> Self;

    /// Add a variable type parameter (?&in)
    pub fn param_any_in(mut self, name: &str) -> Self;

    /// Add a variable type parameter (?&out)
    pub fn param_any_out(mut self, name: &str) -> Self;

    /// Add a parameter with a default value (expression string parsed during apply)
    pub fn param_with_default<T: FromScript>(mut self, name: &str, default_expr: &str) -> Self;

    /// Set return type
    pub fn returns<T: ToScript>(mut self) -> Self;

    /// Set the native implementation
    pub fn native<F>(mut self, f: F) -> Self
    where
        F: NativeCallable + Send + Sync + 'static;

    /// Finish building
    pub fn build(self);
}
```

### Class Builder

```rust
// src/native/class.rs

pub struct ClassBuilder<'m, T: NativeType> {
    module: &'m mut Module,
    name: String,
    type_kind: TypeKind,  // Value or Reference
    constructors: Vec<ConstructorDef>,
    methods: Vec<MethodDef>,
    properties: Vec<PropertyDef>,
    operators: Vec<OperatorDef>,
    behaviors: Behaviors,  // Factory, AddRef, Release, etc.
    _marker: PhantomData<T>,
}

/// Type kind determines memory semantics
pub enum TypeKind {
    /// Value type - stack allocated, copied on assignment
    /// Requires: constructor, destructor, copy/assignment behaviors
    Value {
        /// Size in bytes for stack allocation
        size: usize,
        /// Alignment requirement
        align: usize,
    },
    /// Reference type - heap allocated via factory, handle semantics
    /// Requires: factory, addref, release behaviors
    Reference,
}

/// Object behaviors for lifecycle management (registered but executed by VM)
pub struct Behaviors {
    /// Factory function - creates new instance (reference types)
    pub factory: Option<NativeFn>,
    /// AddRef - increment reference count (reference types)
    pub addref: Option<NativeFn>,
    /// Release - decrement reference count, delete if zero (reference types)
    pub release: Option<NativeFn>,
    /// Constructor - initialize value in pre-allocated memory (value types)
    pub construct: Option<NativeFn>,
    /// Destructor - cleanup before deallocation (value types)
    pub destruct: Option<NativeFn>,
    /// Copy constructor - initialize from another instance (value types)
    pub copy_construct: Option<NativeFn>,
    /// Assignment - copy contents from another instance
    pub assign: Option<NativeFn>,
}

impl<'m, T: NativeType> ClassBuilder<'m, T> {
    /// Mark as value type (default) - stack allocated, copied on assignment
    /// Size and alignment are inferred from T
    pub fn value_type(mut self) -> Self;

    /// Mark as reference type - heap allocated, handle semantics
    pub fn reference_type(mut self) -> Self;

    /// Register a factory function (reference types)
    pub fn factory<F>(mut self, f: F) -> Self
    where F: Fn() -> T + Send + Sync + 'static;

    /// Register AddRef behavior (reference types)
    pub fn addref<F>(mut self, f: F) -> Self
    where F: Fn(&T) + Send + Sync + 'static;

    /// Register Release behavior (reference types)
    pub fn release<F>(mut self, f: F) -> Self
    where F: Fn(&T) + Send + Sync + 'static;

    /// Register a constructor (value types)
    pub fn constructor<Args>(mut self) -> ConstructorBuilder<'m, T, Args>;

    /// Register destructor (value types)
    pub fn destructor<F>(mut self, f: F) -> Self
    where F: Fn(&mut T) + Send + Sync + 'static;

    /// Register a method
    pub fn method(mut self, name: &str) -> MethodBuilder<'m, T>;

    /// Register a const method
    pub fn const_method(mut self, name: &str) -> MethodBuilder<'m, T>;

    /// Register a property with getter
    pub fn property(mut self, name: &str) -> PropertyBuilder<'m, T>;

    /// Register an operator
    pub fn operator(mut self, op: OperatorBehavior) -> OperatorBuilder<'m, T>;

    /// Finish building
    pub fn build(self);
}

/// Builder for property accessors
pub struct PropertyBuilder<'m, T: NativeType> {
    class: &'m mut ClassBuilder<'_, T>,
    name: String,
    getter: Option<NativeFn>,
    setter: Option<NativeFn>,
    prop_type: Option<TypeSpec>,
}

impl<'m, T: NativeType> PropertyBuilder<'m, T> {
    /// Set the getter function (makes property readable)
    pub fn getter<V, F>(mut self, f: F) -> Self
    where
        V: ToScript,
        F: Fn(&T) -> V + Send + Sync + 'static;

    /// Set the setter function (makes property writable)
    pub fn setter<V, F>(mut self, f: F) -> Self
    where
        V: FromScript,
        F: Fn(&mut T, V) + Send + Sync + 'static;

    /// Finish building the property
    pub fn done(self);
}
```

### Enum Builder

```rust
// src/native/enum_builder.rs

pub struct EnumBuilder<'m> {
    module: &'m mut Module,
    name: String,
    values: Vec<(String, i64)>,
}

impl<'m> EnumBuilder<'m> {
    /// Add an enum value
    pub fn value(mut self, name: &str, val: i64) -> Self;

    /// Add consecutive values starting from 0
    pub fn values(mut self, names: &[&str]) -> Self;

    /// Finish building
    pub fn build(self);
}
```

### Interface Builder

```rust
// src/native/interface.rs

pub struct InterfaceBuilder<'m> {
    module: &'m mut Module,
    name: String,
    methods: Vec<MethodSignature>,
}

impl<'m> InterfaceBuilder<'m> {
    /// Add a method signature
    pub fn method(&mut self, name: &str) -> InterfaceMethodBuilder<'_>;

    /// Finish building
    pub fn build(self);
}

pub struct InterfaceMethodBuilder<'i> {
    interface: &'i mut InterfaceBuilder<'_>,
    name: String,
    params: Vec<TypeSpec>,
    return_type: TypeSpec,
    is_const: bool,
}

impl InterfaceMethodBuilder<'_> {
    pub fn param<T: FromScript>(mut self) -> Self;
    pub fn returns<T: ToScript>(mut self) -> Self;
    pub fn is_const(mut self) -> Self;
    pub fn done(self);  // returns to interface builder
}
```

### Funcdef Builder

```rust
// src/native/funcdef.rs

pub struct FuncdefBuilder<'m> {
    module: &'m mut Module,
    name: String,
    params: Vec<TypeSpec>,
    return_type: TypeSpec,
}

impl<'m> FuncdefBuilder<'m> {
    /// Add a parameter type
    pub fn param<T: FromScript>(mut self) -> Self;

    /// Set return type
    pub fn returns<T: ToScript>(mut self) -> Self;

    /// Finish building
    pub fn build(self);
}
```

### Template Builder

```rust
// src/native/template.rs

/// Information about a template instantiation for validation callback
pub struct TemplateInstanceInfo {
    /// The template name
    pub template_name: String,
    /// The type arguments (e.g., [int] for array<int>)
    pub sub_types: Vec<TypeSpec>,
}

/// Result of template validation callback
pub struct TemplateValidation {
    /// Is this instantiation valid?
    pub is_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Should this instance use garbage collection?
    pub needs_gc: bool,
}

pub struct TemplateBuilder<'m> {
    module: &'m mut Module,
    name: String,
    param_count: usize,
    validator: Option<Box<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,
    instance_builder: Option<Box<dyn Fn(&mut TemplateInstanceBuilder, &[TypeSpec]) + Send + Sync>>,
}

impl<'m> TemplateBuilder<'m> {
    /// Set number of type parameters
    pub fn params(mut self, count: usize) -> Self;

    /// Set validation callback (called at compile time)
    pub fn validator<F>(mut self, f: F) -> Self
    where
        F: Fn(&TemplateInstanceInfo) -> TemplateValidation + 'static;

    /// Set instance builder callback (called when template is instantiated)
    pub fn on_instantiate<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut TemplateInstanceBuilder, &[DataType]) + 'static;

    /// Finish building
    pub fn build(self);
}

/// Builder for configuring a template instance's methods
pub struct TemplateInstanceBuilder {
    methods: Vec<InstanceMethod>,
    operators: Vec<(OperatorBehavior, InstanceMethod)>,
    properties: Vec<InstanceProperty>,
}

impl TemplateInstanceBuilder {
    /// Add a method. Use `SubType(0)`, `SubType(1)` for type parameters
    pub fn method(&mut self, name: &str) -> InstanceMethodBuilder<'_>;

    /// Add an operator
    pub fn operator(&mut self, op: OperatorBehavior) -> InstanceOperatorBuilder<'_>;

    /// Add a property
    pub fn property(&mut self, name: &str) -> InstancePropertyBuilder<'_>;
}

/// Placeholder for template type parameter in instance methods
pub struct SubType(pub usize);
```

---

## Template Type Parameter Handling

For template methods that use the element type (like `array<T>.insertLast(T)`):

```rust
/// Represents a type that may be a template parameter
pub enum TypeOrSubType {
    /// A concrete type
    Concrete(DataType),
    /// A template type parameter (index into sub_types)
    SubType(usize),
}

impl TemplateInstanceBuilder {
    pub fn method(&mut self, name: &str) -> InstanceMethodBuilder<'_> {
        InstanceMethodBuilder { ... }
    }
}

impl InstanceMethodBuilder<'_> {
    /// Parameter is a concrete type
    pub fn param<T: FromScript>(self) -> Self;

    /// Parameter is the Nth template type parameter
    pub fn param_subtype(self, index: usize) -> Self;

    /// Return type is a concrete type
    pub fn returns<T: ToScript>(self) -> Self;

    /// Return type is the Nth template type parameter
    pub fn returns_subtype(self, index: usize) -> Self;

    /// Return type is a reference to the Nth template type parameter
    pub fn returns_ref_subtype(self, index: usize) -> Self;
}
```

Example usage for array:

```rust
module.register_template("array")
    .params(1)
    .validator(|info| {
        // Arrays can hold any type
        TemplateValidation { is_valid: true, error: None, needs_gc: false }
    })
    .on_instantiate(|builder, sub_types| {
        // sub_types[0] is the element type (T)
        builder.method("insertLast")
            .param_subtype(0)  // T
            .returns::<()>()
            .native(array_insert_last)
            .done();

        builder.method("length")
            .returns::<u32>()
            .is_const()
            .native(array_length)
            .done();

        builder.operator(OperatorBehavior::OpIndex)
            .param::<i32>()
            .returns_ref_subtype(0)  // &T or &mut T
            .native(array_index)
            .done();
    })
    .build();
```

---

## Module Organization

```
src/
├── context.rs          # Context (owns installed modules)
├── unit.rs             # Unit (compilation unit)
├── module.rs           # Module (namespaced native registrations)
├── native/
│   ├── mod.rs          # Public re-exports (builders, traits)
│   ├── traits.rs       # FromScript, ToScript, NativeType, IntoNativeFn
│   ├── native_fn.rs    # NativeFn, NativeCallable, NativeCallContext
│   ├── error.rs        # NativeError types
│   ├── function.rs     # FunctionBuilder
│   ├── class.rs        # ClassBuilder, MethodBuilder, PropertyBuilder
│   ├── enum_builder.rs # EnumBuilder
│   ├── interface.rs    # InterfaceBuilder
│   ├── funcdef.rs      # FuncdefBuilder
│   ├── template.rs     # TemplateBuilder, TemplateInstanceBuilder
│   ├── any_type.rs     # AnyRef, AnyRefMut for ?& parameters
│   ├── types.rs        # TypeSpec, ParamDef, TypeKind, Behaviors, etc.
│   └── apply.rs        # apply_to_registry() implementation
├── modules/            # Built-in modules (re-exported as angelscript::modules)
│   ├── mod.rs          # default_modules(), individual module exports
│   ├── std.rs          # std() -> Module (print, println, eprint, eprintln)
│   ├── string.rs       # string() -> Module
│   ├── array.rs        # array() -> Module
│   ├── dictionary.rs   # dictionary() -> Module
│   └── math.rs         # math() -> Module (sin, cos, sqrt, etc.)
```

---

## Built-in Types Migration

### String

```rust
// src/modules/string.rs

pub fn string() -> Module {
    let mut module = Module::root();
    module.register_type::<ScriptString>("string")
        .value_type()
        .constructor::<()>()
            .native(|| ScriptString::new())
            .done()
        .constructor::<(&str,)>()
            .native(|s| ScriptString::from(s))
            .done()
        .const_method("length")
            .returns::<u32>()
            .native(|s: &ScriptString| s.len() as u32)
            .done()
        .const_method("substr")
            .param::<u32>("start")
            .param::<i32>("count")
            .returns::<String>()
            .native(ScriptString::substr)
            .done()
        .const_method("findFirst")
            .param::<&str>("str")
            .returns::<i32>()
            .native(ScriptString::find_first)
            .done()
        .const_method("isEmpty")
            .returns::<bool>()
            .native(|s: &ScriptString| s.is_empty())
            .done()
        .operator(OperatorBehavior::OpAdd)
            .param::<&str>()
            .returns::<String>()
            .native(|a: &ScriptString, b: &str| a.concat(b))
            .done()
        .operator(OperatorBehavior::OpEquals)
            .param::<&str>()
            .returns::<bool>()
            .native(|a: &ScriptString, b: &str| a.as_str() == b)
            .done()
        .operator(OperatorBehavior::OpIndex)
            .param::<u32>()
            .returns::<u8>()
            .native(|s: &ScriptString, i: u32| s.char_at(i))
            .done()
        .build();
    module
}
```

### Std (Standard Library)

```rust
// src/modules/std.rs

pub fn std() -> Module {
    let mut module = Module::root();

    // Print without newline
    module.register_fn("print", |s: &str| {
        print!("{}", s);
    });

    // Print with newline
    module.register_fn("println", |s: &str| {
        println!("{}", s);
    });

    // Print to stderr without newline
    module.register_fn("eprint", |s: &str| {
        eprint!("{}", s);
    });

    // Print to stderr with newline
    module.register_fn("eprintln", |s: &str| {
        eprintln!("{}", s);
    });

    module
}
```

### Array Template

```rust
// src/modules/array.rs

pub fn array() -> Module {
    let mut module = Module::root();
    module.register_template("array")
        .params(1)
        .validator(|_| TemplateValidation::valid())
        .on_instantiate(|builder, sub_types| {
            let elem_type = &sub_types[0];

            // Constructor
            builder.constructor()
                .native(|| ScriptArray::new())
                .done();
            builder.constructor()
                .param::<u32>()  // initial size
                .native(|size| ScriptArray::with_capacity(size))
                .done();

            // Methods
            builder.const_method("length")
                .returns::<u32>()
                .native(ScriptArray::len)
                .done();

            builder.method("resize")
                .param::<u32>()
                .returns::<()>()
                .native(ScriptArray::resize)
                .done();

            builder.method("insertLast")
                .param_subtype(0)  // T
                .returns::<()>()
                .native_generic(array_insert_last)
                .done();

            builder.method("removeLast")
                .returns::<()>()
                .native(ScriptArray::pop)
                .done();

            // Operators
            builder.operator(OperatorBehavior::OpIndex)
                .param::<i32>()
                .returns_ref_subtype(0)  // &T
                .native_generic(array_index)
                .done();
        })
        .build();
    module
}
```

### Dictionary Template

```rust
// src/modules/dictionary.rs

pub fn dictionary() -> Module {
    let mut module = Module::root();
    module.register_template("dictionary")
        .params(2)  // K, V
        .validator(|info| {
            // Keys must be hashable (primitives, string, handles)
            let key_type = &info.sub_types[0];
            if is_hashable(key_type) {
                TemplateValidation::valid()
            } else {
                TemplateValidation::invalid("Dictionary key must be hashable")
            }
        })
        .on_instantiate(|builder, sub_types| {
            builder.method("set")
                .param_subtype(0)  // K
                .param_subtype(1)  // V
                .returns::<()>()
                .native_generic(dict_set)
                .done();

            builder.const_method("get")
                .param_subtype(0)     // K
                .param_out_subtype(1) // V &out
                .returns::<bool>()
                .native_generic(dict_get)
                .done();

            builder.const_method("exists")
                .param_subtype(0)  // K
                .returns::<bool>()
                .native_generic(dict_exists)
                .done();

            builder.method("delete")
                .param_subtype(0)  // K
                .returns::<bool>()
                .native_generic(dict_delete)
                .done();

            builder.const_method("isEmpty")
                .returns::<bool>()
                .native(ScriptDict::is_empty)
                .done();

            builder.const_method("getSize")
                .returns::<u32>()
                .native(ScriptDict::len)
                .done();
        })
        .build();
    module
}
```

---

## Implementation Phases

### Phase 1: Core Infrastructure
1. Create `src/context.rs` - Context struct (owns installed modules)
2. Create `src/unit.rs` - Unit struct (compilation unit)
3. Create `src/module.rs` - Module struct (namespaced native registrations)
4. Create `src/native/mod.rs` - Public re-exports
5. Create `src/native/types.rs` - TypeSpec, ParamDef, TypeKind, Behaviors, etc.
6. Create `src/native/traits.rs` - FromScript, ToScript, NativeType, IntoNativeFn
7. Create `src/native/native_fn.rs` - NativeFn, NativeCallable
8. Create `src/native/error.rs` - NativeError enum

### Phase 2: Function Registration
9. Create `src/native/function.rs` - FunctionBuilder
10. Implement primitive parameter/return type handling
11. Implement `?&` variable type parameters
12. Implement default parameter support (string expressions)
13. Implement FunctionTraits for const, constructor, etc.

### Phase 3: Class Registration
14. Create `src/native/class.rs` - ClassBuilder, MethodBuilder, PropertyBuilder
15. Implement TypeKind (Value vs Reference)
16. Implement Behaviors (factory, addref, release, construct, destruct)
17. Implement constructors, methods, properties, operators

### Phase 4: Other Type Registrations
18. Create `src/native/enum_builder.rs` - EnumBuilder
19. Create `src/native/interface.rs` - InterfaceBuilder
20. Create `src/native/funcdef.rs` - FuncdefBuilder

### Phase 5: Template Support
21. Create `src/native/template.rs` - TemplateBuilder, TemplateInstanceBuilder
22. Implement validation callbacks
23. Implement SubType parameter handling

### Phase 6: Apply to Registry
24. Create `src/native/apply.rs` - apply_to_registry() implementation
25. Convert TypeSpec → DataType (resolve type names to TypeId)
26. Convert NativeFunctionDef → FunctionDef
27. Parse default parameter expressions into arena
28. Convert NativeTypeDef → TypeDef

### Phase 7: Built-in Modules
29. Create `src/modules/mod.rs` - default_modules(), re-exports
30. Create `src/modules/std.rs` - std() -> Module (print, println, eprint, eprintln)
31. Create `src/modules/string.rs` - string() -> Module
32. Create `src/modules/array.rs` - array() -> Module
33. Create `src/modules/dictionary.rs` - dictionary() -> Module
34. Create `src/modules/math.rs` - math() -> Module
35. Modify `src/semantic/types/registry.rs` - remove hardcoded implementations

### Phase 8: Integration
36. Update `src/lib.rs` - export Context, Unit, Module, builders, traits
37. Update existing module.rs → unit.rs migration
38. Comprehensive tests

---

## Critical Files

**To Create:**
- `src/context.rs` - Context (owns installed modules)
- `src/unit.rs` - Unit (compilation unit)
- `src/module.rs` - Module (namespaced native registrations)
- `src/native/mod.rs` - Public re-exports
- `src/native/types.rs` - TypeSpec, ParamDef, TypeKind, Behaviors, etc.
- `src/native/apply.rs` - apply_to_registry() implementation
- `src/native/traits.rs` - FromScript, ToScript, NativeType, IntoNativeFn
- `src/native/native_fn.rs` - NativeFn, NativeCallable
- `src/native/error.rs` - NativeError types
- `src/native/function.rs` - FunctionBuilder
- `src/native/class.rs` - ClassBuilder, MethodBuilder, PropertyBuilder
- `src/native/enum_builder.rs` - EnumBuilder
- `src/native/interface.rs` - InterfaceBuilder
- `src/native/funcdef.rs` - FuncdefBuilder
- `src/native/template.rs` - TemplateBuilder, TemplateInstanceBuilder
- `src/native/any_type.rs` - AnyRef, AnyRefMut
- `src/modules/mod.rs` - default_modules(), re-exports
- `src/modules/std.rs` - std() -> Module (print, println, eprint, eprintln)
- `src/modules/string.rs` - string() -> Module
- `src/modules/array.rs` - array() -> Module
- `src/modules/dictionary.rs` - dictionary() -> Module
- `src/modules/math.rs` - math() -> Module

**To Modify:**
- `src/semantic/types/registry.rs` - Remove ~800 lines of hardcoded builtins
- `src/lib.rs` - Export Context, Unit, Module, add `pub mod native`, `pub mod modules`
- Rename/refactor existing `src/module.rs` → `src/unit.rs`

---

## Sources

- [AngelScript Template Types](https://www.angelcode.com/angelscript/sdk/docs/manual/doc_adv_template.html)
- [AngelScript Variable Parameter Type](https://angelcode.com/angelscript/sdk/docs/manual/doc_adv_var_type.html)
- [AngelScript Generic Calling Convention](https://angelcode.com/angelscript/sdk/docs/manual/doc_generic.html)
- [AngelScript Function Registration](https://www.angelcode.com/angelscript/sdk/docs/manual/doc_register_func.html)
- [AngelScript Type Registration](https://www.angelcode.com/angelscript/sdk/docs/manual/doc_register_type.html)
- [Rhai Custom Types](https://rhai.rs/book/rust/custom-types.html)
- [Rhai Function Registration](https://rhai.rs/book/rust/functions.html)
- [Rune Documentation](https://docs.rs/rune)
