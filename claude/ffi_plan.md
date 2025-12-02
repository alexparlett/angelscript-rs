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

1. **Declaration string API** - parse AngelScript syntax for registration metadata
2. **AST reuse** - leverage existing `Parser`, `Lexer`, and AST types (`TypeExpr`, `Ident`, `FunctionParam`)
3. **Function pointers stored for semantic analysis** - not just signatures
4. **Template callback pattern** inspired by AngelScript (unified under register_type)
5. **Full coverage**: enums, interfaces, funcdefs, templates, variadics
6. **Shareable across modules** - FFI definitions can be reused
7. **All registration methods return Result** - handle parse errors at registration time

### AST Reuse Strategy

**Key architectural decision:** We reuse the existing AST parser infrastructure rather than creating a separate parsing system for FFI declarations.

**Benefits:**
- Single source of truth for AngelScript syntax
- Consistent semantics between script code and FFI declarations
- Less code to maintain
- Arena allocation (`Bump`) already handles memory efficiently

**Parser Analysis:**

Looking at the existing parser (`src/ast/decl_parser.rs`):

| Parser Method | Terminator | FFI Status |
|---------------|------------|------------|
| `parse_enum` | `}` | ❌ Not needed (using builder) |
| `parse_interface` | `}` | ❌ Not needed (using builder) |
| `parse_funcdef` | `;` (line 2531) | ⚠️ Needs refactoring |
| `parse_function_or_global_var` | `;` (lines 372, 418) | ⚠️ Needs refactoring |

**Key constraint:** Normal script parsing must not regress - semicolons are still required where the language expects them.

**Refactoring approach:**
1. Extract internal parsing methods from `parse_funcdef` and `parse_function_or_global_var`
2. Existing script parsing continues to call internal method + expect semicolon
3. New FFI entry points call internal method + expect EOF

```rust
impl<'ast> Parser<'ast> {
    // ═══════════════════════════════════════════════════════════════════
    // Function signatures - needs refactoring
    // ═══════════════════════════════════════════════════════════════════

    /// New internal method - parses signature without terminator
    fn parse_function_signature_inner(&mut self) -> Result<FunctionSignatureData<'ast>, ParseError> {
        let return_type = self.parse_return_type()?;
        let name = self.parse_ident()?;
        let params = self.parse_function_params()?;
        let is_const = self.eat(TokenKind::Const).is_some();
        let attrs = self.parse_func_attrs()?;
        Ok(FunctionSignatureData { return_type, name, params, is_const, attrs })
    }

    // Existing parse_function_or_global_var calls internal + handles body/semicolon

    /// FFI entry point - accepts EOF
    pub fn parse_ffi_function_signature(&mut self) -> Result<FunctionSignatureData<'ast>, ParseError> {
        let sig = self.parse_function_signature_inner()?;
        self.expect_eof()?;
        Ok(sig)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Funcdef - needs refactoring
    // ═══════════════════════════════════════════════════════════════════

    /// New internal method - parses funcdef without semicolon
    fn parse_funcdef_inner(&mut self, modifiers: DeclModifiers) -> Result<FuncdefDecl<'ast>, ParseError>;

    // Existing parse_funcdef calls internal + expects semicolon

    /// FFI entry point - accepts EOF
    pub fn parse_ffi_funcdef(&mut self) -> Result<FuncdefDecl<'ast>, ParseError> {
        let decl = self.parse_funcdef_inner(DeclModifiers::new())?;
        self.expect_eof()?;
        Ok(decl)
    }

    // ═══════════════════════════════════════════════════════════════════
    // Enum/Interface - NOT NEEDED (using builder pattern)
    // ═══════════════════════════════════════════════════════════════════

    // EnumBuilder and InterfaceBuilder don't need any parsing - they use
    // simple string names for values and method signature parsing respectively.

    /// Helper: expect end of input for FFI parsing
    fn expect_eof(&mut self) -> Result<(), ParseError> {
        if !self.is_eof() {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                self.peek().span,
                "expected end of declaration",
            ));
        }
        Ok(())
    }
}
```

**Implementation:**
- Module owns a `Bump` arena for storing parsed AST nodes
- New `Parser` entry points for partial declarations (no semicolons required)
- Template context passed to parser so `T`, `K`, `V` are recognized as type placeholders
- Output uses existing AST types: `TypeExpr<'ast>`, `Ident<'ast>`, `FunctionParam<'ast>`

```rust
// Module owns arena, parsed nodes live there
pub struct Module<'app> {
    namespace: Vec<String>,
    arena: Bump,
    functions: Vec<NativeFunctionDef<'_>>,  // Contains &'ast TypeExpr, etc.
    // ...
}

// Parsing uses existing infrastructure - no semicolons needed
impl<'app> Module<'app> {
    fn parse_fn_decl(&self, decl: &str) -> Result<FunctionSignature<'_>, FfiRegistrationError> {
        let lexer = Lexer::new(decl, "ffi");
        let mut parser = Parser::new(lexer, &self.arena);
        parser.parse_function_signature()  // Accepts "float sqrt(float x)" directly
            .map_err(|e| FfiRegistrationError::ParseError { decl: decl.into(), error: e.to_string() })
    }
}
```

### Top-Level API: Context + Module

Inspired by Rune's module system, native registrations are organized into **Module**s that each have a namespace. Modules are installed into a `Context`, and scripts access items via their namespace.

**Module** - A namespaced collection of native functions/types:
```rust
// Create a module with a namespace
let mut math = Module::new(&["math"]);
math.register_fn("float sqrt(float x)", |x: f64| x.sqrt())?;
math.register_fn("float sin(float x)", |x: f64| x.sin())?;
math.register_type::<Vec3>("Vec3")
    .value_type()
    .method("float length() const", Vec3::length)?
    .build()?;

// Nested namespaces using array syntax
let mut collections = Module::new(&["std", "collections"]);
collections.register_type::<ScriptHashMap>("HashMap<class K, class V>")
    .reference_type()
    .template_callback(|info| TemplateValidation::valid())?
    .method("void set(const K &in, const V &in)", hashmap_set)?
    .build()?;
// In script: std::collections::HashMap<string, int>

// More nesting examples
let mut game_physics = Module::new(&["game", "physics"]);
game_physics.register_type::<RigidBody>("RigidBody")
    .reference_type()
    .build()?;
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
globals.register_fn("void print(const string &in s)", |s: &str| println!("{}", s))?;
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
│  ├── register_fn(decl, f) → Result                          │
│  ├── register_type<T>(decl) → ClassBuilder                  │
│  ├── register_enum(decl) → Result                           │
│  ├── register_interface(decl) → Result                      │
│  └── register_funcdef(decl) → Result                        │
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
| **Type-Safe** | `register_fn`, `method` | Simple functions, known types | Declaration string parsed |
| **Generic** | `register_fn_raw`, `method_raw` | `?&` params, complex logic | Declaration string parsed |

**Why both?**
- Type-safe is ergonomic for 90% of cases - just pass a closure
- Generic is required for `?&` (variable type) parameters where type isn't known at registration
- Generic gives full control for performance-critical code or complex conditional logic

Both conventions store a `NativeFn` internally - the difference is in how arguments are extracted:

**1. Type-Safe (High-Level)** - Idiomatic Rust signatures with automatic conversion:
```rust
// Global function - declaration string + Rust closure
module.register_fn("float sqrt(float x)", |x: f64| x.sqrt())?;
module.register_fn("bool contains(const string &in s, const string &in needle)",
    |s: &ScriptString, needle: &str| s.as_str().contains(needle))?;

// Method - self is first parameter, inferred from ClassBuilder<T>
module.register_type::<Vec3>("Vec3")
    .value_type()
    .method("float length() const", |this: &Vec3| this.length())?        // &self
    .method("void normalize()", |this: &mut Vec3| this.normalize())?     // &mut self
    .method("Vec3 add(const Vec3 &in other) const", |this: &Vec3, other: &Vec3| *this + *other)?
    .build()?;
```

**2. Generic (Low-Level)** - Manual argument extraction for complex cases:
```rust
// For ?& parameters, complex types, or full control
module.register_fn_raw("string format(const string &in fmt, ?&in value)",
    |ctx: &mut CallContext| -> Result<(), NativeError> {
        let fmt: &str = ctx.arg::<&str>(0)?;
        let any_val = ctx.arg_any(1)?;  // ?&in - returns AnyRef
        let type_id = any_val.type_id();
        // ... format based on type
        ctx.set_return(result);
        Ok(())
    })?;

// Methods with raw context
module.register_type::<Foo>("Foo")
    .value_type()
    .method_raw("void complex(?&in value)", |ctx: &mut CallContext| {
        let this: &Foo = ctx.this()?;  // Get self reference
        let arg = ctx.arg_any(0)?;      // ?&in parameter
        // ...
        Ok(())
    })?
    .build()?;
```

**Signature Declaration:**
- Both type-safe and generic use declaration strings parsed at registration time
- Declaration string is AngelScript syntax: `"ReturnType name(ParamType param, ...)"`
- Parse errors are returned as `Result::Err` at registration time

**Self Handling for Methods:**

For type-safe methods, `self` is always the first parameter in the closure:
```rust
// These are equivalent:
.method("float length() const", |this: &Vec3| this.length())?
.method("float length() const", Vec3::length)?  // fn(&self) -> f32

// The ClassBuilder knows T, so it can:
// 1. Extract `this` from VM's first argument slot
// 2. Cast it to &T or &mut T based on closure signature
// 3. Pass remaining VM args to the closure's other parameters
```

For raw methods, use `ctx.this::<T>()`:
```rust
.method_raw("void foo()", |ctx: &mut CallContext| {
    let this: &Foo = ctx.this()?;        // Immutable borrow
    let this: &mut Foo = ctx.this_mut()?; // Mutable borrow
    Ok(())
})?;
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
    /// Declaration string format: "ReturnType name(ParamType param, ...)"
    pub fn register_fn<F, Args, Ret>(
        &mut self,
        decl: &str,
        f: F,
    ) -> Result<&mut Self, FfiRegistrationError>
    where
        F: IntoNativeFn<Args, Ret>;

    /// Register a global native function with raw CallContext.
    pub fn register_fn_raw(
        &mut self,
        decl: &str,
        f: impl NativeCallable,
    ) -> Result<&mut Self, FfiRegistrationError>;

    /// Register a global property. The app owns the data; scripts read/write via reference.
    /// Declaration string format: "[const] Type name"
    pub fn register_global_property<T: 'static>(
        &mut self,
        decl: &str,
        value: &'app mut T,
    ) -> Result<&mut Self, FfiRegistrationError>;

    /// Register a native class type (or template with <class T> syntax).
    /// For templates: "array<class T>", "dictionary<class K, class V>"
    pub fn register_type<T: NativeType>(&mut self, name: &str) -> ClassBuilder<'_, 'app, T>;

    /// Register a native enum, returning a builder.
    /// Use builder methods to add values, then call build().
    pub fn register_enum(&mut self, name: &str) -> EnumBuilder<'_, 'app>;

    /// Register a native interface, returning a builder.
    /// Use builder methods to add method signatures, then call build().
    pub fn register_interface(&mut self, name: &str) -> InterfaceBuilder<'_, 'app>;

    /// Register a funcdef (function pointer type).
    /// Declaration string format: "funcdef ReturnType Name(ParamType, ...)"
    pub fn register_funcdef(&mut self, decl: &str) -> Result<&mut Self, FfiRegistrationError>;
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

### Function Registration

Functions are registered directly on Module with declaration strings - no separate FunctionBuilder needed:

```rust
// Type-safe registration
module.register_fn("float sqrt(float x)", |x: f32| x.sqrt())?;
module.register_fn("void print(const string &in s)", |s: &str| println!("{}", s))?;
module.register_fn("int max(int a, int b)", |a: i32, b: i32| a.max(b))?;

// With default arguments (parsed from declaration)
module.register_fn("void log(const string &in msg, int level = 0)", log_fn)?;

// Raw/generic for ?& parameters
module.register_fn_raw("string format(const string &in fmt, ?&in value)", |ctx| {
    let fmt: &str = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    // ...
    Ok(())
})?;
```

### Class Builder

```rust
// src/ffi/class.rs

pub struct ClassBuilder<'m, 'app, T: NativeType> {
    module: &'m mut Module<'app>,
    name: String,
    template_params: Option<Vec<String>>,  // For templates: ["T"] or ["K", "V"]
    type_kind: TypeKind,
    // ... internal storage
}

impl<'m, 'app, T: NativeType> ClassBuilder<'m, 'app, T> {
    /// Mark as value type (default) - stack allocated, copied on assignment
    pub fn value_type(mut self) -> Self;

    /// Mark as reference type - heap allocated, handle semantics
    pub fn reference_type(mut self) -> Self;

    /// Register template validation callback (for template types)
    pub fn template_callback<F>(mut self, f: F) -> Result<Self, FfiRegistrationError>
    where F: Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync + 'static;

    /// Register a constructor (value types)
    /// Declaration: "void f()" or "void f(int x, int y)"
    pub fn constructor<F, Args>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<Args, T>;

    /// Register a factory (reference types)
    /// Declaration: "T@ f()" or "T@ f(const string &in name)"
    pub fn factory<F, Args>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<Args, T>;

    /// Register AddRef behavior (reference types)
    pub fn addref<F>(mut self, f: F) -> Self
    where F: Fn(&T) + Send + Sync + 'static;

    /// Register Release behavior (reference types)
    pub fn release<F>(mut self, f: F) -> Self
    where F: Fn(&T) + Send + Sync + 'static;

    /// Register destructor (value types)
    pub fn destructor<F>(mut self, f: F) -> Self
    where F: Fn(&mut T) + Send + Sync + 'static;

    /// Register a method
    /// Declaration: "ReturnType name(params) [const]"
    pub fn method<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<(&T, Args), Ret>;

    /// Register a method with raw CallContext
    pub fn method_raw<F>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: NativeCallable + Send + Sync + 'static;

    /// Register a read-only property
    /// Declaration: "Type name"
    pub fn property_get<V, F>(mut self, decl: &str, getter: F) -> Result<Self, FfiRegistrationError>
    where
        V: ToScript,
        F: Fn(&T) -> V + Send + Sync + 'static;

    /// Register a read-write property
    /// Declaration: "Type name"
    pub fn property<V, G, S>(mut self, decl: &str, getter: G, setter: S) -> Result<Self, FfiRegistrationError>
    where
        V: ToScript + FromScript,
        G: Fn(&T) -> V + Send + Sync + 'static,
        S: Fn(&mut T, V) + Send + Sync + 'static;

    /// Register an operator
    /// Declaration: "ReturnType opName(params)" e.g. "Vec3 opAdd(const Vec3 &in)"
    pub fn operator<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<(&T, Args), Ret>;

    /// Finish building
    pub fn build(self) -> Result<(), FfiRegistrationError>;
}
```

**Usage Examples:**

```rust
// Value type
module.register_type::<Vec3>("Vec3")
    .value_type()
    .constructor("void f()", || Vec3::default())?
    .constructor("void f(float x, float y, float z)", Vec3::new)?
    .method("float length() const", |v: &Vec3| v.length())?
    .method("void normalize()", |v: &mut Vec3| v.normalize())?
    .property("float x", |v| v.x, |v, x| v.x = x)?
    .property_get("float lengthSq", |v| v.length_squared())?
    .operator("Vec3 opAdd(const Vec3 &in)", |a, b| *a + *b)?
    .operator("bool opEquals(const Vec3 &in)", |a, b| a == b)?
    .build()?;

// Reference type
module.register_type::<Entity>("Entity")
    .reference_type()
    .factory("Entity@ f()", || Entity::new())?
    .factory("Entity@ f(const string &in name)", Entity::with_name)?
    .addref(Entity::add_ref)
    .release(Entity::release)
    .method("string getName() const", |e| e.name.clone())?
    .method("void setName(const string &in)", |e, name| e.name = name)?
    .build()?;

// Template type (unified under register_type)
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .template_callback(|info| TemplateValidation::valid())?
    .factory("array<T>@ f()", || ScriptArray::new())?
    .factory("array<T>@ f(uint size)", ScriptArray::with_capacity)?
    .method("void insertLast(const T &in)", array_insert_last)?
    .method("uint length() const", ScriptArray::len)?
    .operator("T& opIndex(uint)", array_index)?
    .build()?;
```

### Enum, Interface, and Funcdef Registration

Enums and interfaces use builder patterns (like `ClassBuilder`), while funcdefs use declaration string parsing:

```rust
// Enums - builder pattern
module.register_enum("Color")
    .value("Red", 0)?
    .value("Green", 1)?
    .value("Blue", 2)?
    .build()?;

module.register_enum("Direction")
    .auto_value("North")?   // 0
    .auto_value("East")?    // 1
    .auto_value("South")?   // 2
    .auto_value("West")?    // 3
    .build()?;

module.register_enum("Flags")
    .value("None", 0)?
    .value("Read", 1)?
    .value("Write", 2)?
    .value("Execute", 4)?
    .build()?;

// Interfaces - builder pattern with method declaration strings
module.register_interface("IDrawable")
    .method("void draw() const")?
    .method("void setVisible(bool)")?
    .build()?;

module.register_interface("ISerializable")
    .method("string serialize() const")?
    .method("void deserialize(const string &in data)")?
    .build()?;

// Funcdefs - declaration string parsing
module.register_funcdef("funcdef void Callback()")?;
module.register_funcdef("funcdef bool Predicate(int value)")?;
module.register_funcdef("funcdef void EventHandler(const string &in event, ?&in data)")?;
```

### Template Type Parameter Handling

Templates are registered using `register_type` with `<class T>` syntax. The parser extracts template parameter names from the type declaration and recognizes them in method signatures:

```rust
// Single type parameter - T is recognized in method signatures
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .template_callback(|info| TemplateValidation::valid())?
    .factory("array<T>@ f()", || ScriptArray::new())?
    .method("void insertLast(const T &in)", array_insert_last)?
    .method("uint length() const", ScriptArray::len)?
    .operator("T& opIndex(uint)", array_index)?
    .build()?;

// Multiple type parameters - K and V are recognized
module.register_type::<ScriptDict>("dictionary<class K, class V>")
    .reference_type()
    .template_callback(|info| {
        if is_hashable(&info.sub_types[0]) {
            TemplateValidation::valid()
        } else {
            TemplateValidation::invalid("Key must be hashable")
        }
    })?
    .method("void set(const K &in, const V &in)", dict_set)?
    .method("V& opIndex(const K &in)", dict_index)?
    .build()?;
```

**TemplateInstanceInfo and TemplateValidation:**

```rust
/// Information about a template instantiation for validation callback
pub struct TemplateInstanceInfo {
    /// The template name (e.g., "array")
    pub template_name: String,
    /// The type arguments (e.g., [int] for array<int>)
    pub sub_types: Vec<DataType>,
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

impl TemplateValidation {
    pub fn valid() -> Self;
    pub fn invalid(msg: &str) -> Self;
    pub fn with_gc() -> Self;  // Valid and needs garbage collection
}
```

---

## Module Organization

```
src/
├── context.rs          # Context (owns installed modules)
├── unit.rs             # Unit (compilation unit, replaces current module.rs)
├── ffi/                # FFI registration system
│   ├── mod.rs          # Public re-exports (Module, ClassBuilder, traits)
│   ├── module.rs       # Module<'app> struct (namespaced native registrations)
│   ├── traits.rs       # FromScript, ToScript, NativeType, IntoNativeFn
│   ├── native_fn.rs    # NativeFn, NativeCallable, CallContext
│   ├── error.rs        # NativeError, FfiRegistrationError
│   ├── class.rs        # ClassBuilder (value/ref types with declaration string methods)
│   ├── enum_builder.rs # EnumBuilder for enum registration
│   ├── interface_builder.rs # InterfaceBuilder for interface registration
│   ├── any_type.rs     # AnyRef, AnyRefMut for ?& parameters
│   ├── types.rs        # TypeKind, Behaviors, TemplateInstanceInfo, TemplateValidation
│   ├── global_property.rs # GlobalPropertyDef, GlobalPropertyRef
│   └── apply.rs        # apply_to_registry() implementation
├── modules/            # Built-in modules (re-exported as angelscript::modules)
│   ├── mod.rs          # default_modules(), individual module exports
│   ├── std.rs          # std() -> Module (print, println, eprint, eprintln)
│   ├── string.rs       # string() -> Module
│   ├── array.rs        # array() -> Module
│   ├── dictionary.rs   # dictionary() -> Module
│   └── math.rs         # math() -> Module (sin, cos, sqrt, etc.)
├── benches/
│   └── module_benchmarks.rs  # Updated to use Context/Unit API
└── tests/
    ├── module_tests.rs       # Updated to use Context/Unit API
    └── test_harness.rs       # Updated to use Context/Unit API
```

**Test Scripts with FFI Placeholders to Clean Up:**
```
test_scripts/
├── hello_world.as          # void print(const string &in) {}
├── expressions.as
├── utilities.as
├── interface.as
├── using_namespace.as
├── enum.as
├── templates.as
├── game_logic.as
├── inheritance.as
├── data_structures.as
├── nested.as
├── functions.as
├── literals.as
├── class_basic.as
├── control_flow.as
├── types.as
└── performance/
    ├── large_500.as
    ├── xlarge_1000.as
    └── xxlarge_5000.as
```

---

## Built-in Types Migration

### String

```rust
// src/modules/string.rs

pub fn string() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();
    module.register_type::<ScriptString>("string")
        .value_type()
        .constructor("void f()", || ScriptString::new())?
        .constructor("void f(const string &in s)", ScriptString::from)?
        .method("uint length() const", |s: &ScriptString| s.len() as u32)?
        .method("string substr(uint start, int count = -1) const", ScriptString::substr)?
        .method("int findFirst(const string &in str, uint start = 0) const", ScriptString::find_first)?
        .method("bool isEmpty() const", |s: &ScriptString| s.is_empty())?
        .operator("string opAdd(const string &in)", |a: &ScriptString, b: &str| a.concat(b))?
        .operator("bool opEquals(const string &in)", |a: &ScriptString, b: &str| a.as_str() == b)?
        .operator("uint8 opIndex(uint)", |s: &ScriptString, i: u32| s.char_at(i))?
        .build()?;
    Ok(module)
}
```

### Std (Standard Library)

```rust
// src/modules/std.rs

pub fn std() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();

    // Print without newline
    module.register_fn("void print(const string &in s)", |s: &str| {
        print!("{}", s);
    })?;

    // Print with newline
    module.register_fn("void println(const string &in s)", |s: &str| {
        println!("{}", s);
    })?;

    // Print to stderr without newline
    module.register_fn("void eprint(const string &in s)", |s: &str| {
        eprint!("{}", s);
    })?;

    // Print to stderr with newline
    module.register_fn("void eprintln(const string &in s)", |s: &str| {
        eprintln!("{}", s);
    })?;

    Ok(module)
}
```

### Array Template

```rust
// src/modules/array.rs

pub fn array() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();
    module.register_type::<ScriptArray>("array<class T>")
        .reference_type()
        .template_callback(|_| TemplateValidation::valid())?
        .factory("array<T>@ f()", || ScriptArray::new())?
        .factory("array<T>@ f(uint size)", ScriptArray::with_capacity)?
        .method("uint length() const", ScriptArray::len)?
        .method("void resize(uint size)", ScriptArray::resize)?
        .method("void insertLast(const T &in value)", array_insert_last)?
        .method("void removeLast()", ScriptArray::pop)?
        .operator("T& opIndex(int index)", array_index)?
        .build()?;
    Ok(module)
}
```

### Dictionary Template

```rust
// src/modules/dictionary.rs

pub fn dictionary() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();
    module.register_type::<ScriptDict>("dictionary<class K, class V>")
        .reference_type()
        .template_callback(|info| {
            // Keys must be hashable (primitives, string, handles)
            let key_type = &info.sub_types[0];
            if is_hashable(key_type) {
                TemplateValidation::valid()
            } else {
                TemplateValidation::invalid("Dictionary key must be hashable")
            }
        })?
        .factory("dictionary<K,V>@ f()", || ScriptDict::new())?
        .method("void set(const K &in key, const V &in value)", dict_set)?
        .method("bool get(const K &in key, V &out value) const", dict_get)?
        .method("bool exists(const K &in key) const", dict_exists)?
        .method("bool delete(const K &in key)", dict_delete)?
        .method("bool isEmpty() const", ScriptDict::is_empty)?
        .method("uint getSize() const", ScriptDict::len)?
        .build()?;
    Ok(module)
}
```

---

## Implementation Tasks

Detailed task files are in `/claude/tasks/`. Complete in order:

### Phase 1: Core Infrastructure

| Task | File | Description |
|------|------|-------------|
| 01 | [01_ffi_core_infrastructure.md](tasks/01_ffi_core_infrastructure.md) | Core types, traits (FromScript, ToScript, NativeType, CallContext, NativeFn) |
| 02 | [02_ffi_module_and_context.md](tasks/02_ffi_module_and_context.md) | Module<'app> and Context API, GlobalPropertyDef |

### Phase 2: Registration API

| Task | File | Description |
|------|------|-------------|
| 03 | [03_ffi_function_registration.md](tasks/03_ffi_function_registration.md) | Module.register_fn() with declaration string parsing |
| 04 | [04_ffi_class_builder.md](tasks/04_ffi_class_builder.md) | ClassBuilder for value/ref/template types with declaration string methods |
| 05 | [05_ffi_enum_interface_funcdef.md](tasks/05_ffi_enum_interface_funcdef.md) | Module.register_enum/interface/funcdef with declaration parsing |

### Phase 3: Integration

| Task | File | Description |
|------|------|-------------|
| 07 | [07_ffi_apply_to_registry.md](tasks/07_ffi_apply_to_registry.md) | apply_to_registry() - convert FFI registrations to Registry entries |
| 08 | [08_ffi_builtin_modules.md](tasks/08_ffi_builtin_modules.md) | Implement std, string, array, dictionary, math modules via FFI |

### Phase 4: Migration

| Task | File | Description |
|------|------|-------------|
| 09 | [09_ffi_update_entry_points.md](tasks/09_ffi_update_entry_points.md) | Update benches and tests to use Context/Unit API |
| 10 | [10_ffi_extract_placeholders.md](tasks/10_ffi_extract_placeholders.md) | Remove FFI placeholders from 19 test scripts |
| 11 | [11_ffi_lib_exports.md](tasks/11_ffi_lib_exports.md) | Library exports and public API organization |

---

## Critical Files

**To Create (FFI Core):**
- `src/context.rs` - Context (owns installed modules)
- `src/unit.rs` - Unit (compilation unit)
- `src/ffi/mod.rs` - Public re-exports (Module, ClassBuilder, EnumBuilder, InterfaceBuilder, traits)
- `src/ffi/module.rs` - Module<'app> with register_fn, register_type, register_enum, etc.
- `src/ffi/types.rs` - TypeKind, Behaviors, TemplateInstanceInfo, TemplateValidation
- `src/ffi/traits.rs` - FromScript, ToScript, NativeType, IntoNativeFn
- `src/ffi/native_fn.rs` - NativeFn, NativeCallable, CallContext
- `src/ffi/error.rs` - NativeError, FfiRegistrationError
- `src/ffi/global_property.rs` - GlobalPropertyDef, GlobalPropertyRef
- `src/ffi/class.rs` - ClassBuilder with declaration string methods
- `src/ffi/enum_builder.rs` - EnumBuilder for enum registration
- `src/ffi/interface_builder.rs` - InterfaceBuilder for interface registration
- `src/ffi/any_type.rs` - AnyRef, AnyRefMut
- `src/ffi/apply.rs` - apply_to_registry() implementation

**To Create (Built-in Modules):**
- `src/modules/mod.rs` - default_modules(), re-exports
- `src/modules/std.rs` - std() -> Module (print, println, eprint, eprintln)
- `src/modules/string.rs` - string() -> Module
- `src/modules/array.rs` - array() -> Module
- `src/modules/dictionary.rs` - dictionary() -> Module
- `src/modules/math.rs` - math() -> Module

**To Modify (Integration):**
- `src/semantic/types/registry.rs` - Remove ~800 lines of hardcoded builtins
- `src/lib.rs` - Export Context, Unit, add `pub mod ffi`, `pub mod modules`
- Rename/refactor existing `src/module.rs` → `src/unit.rs`

**To Modify (Migration):**
- `benches/module_benchmarks.rs` - Update to use Context/Unit API
- `tests/module_tests.rs` - Update to use Context/Unit API
- `tests/test_harness.rs` - Update to use Context/Unit API

**To Clean Up (FFI Placeholders):**
Remove placeholder function stubs from 19 test scripts:
- `test_scripts/hello_world.as` - Remove `void print(const string &in) {}`
- `test_scripts/expressions.as`
- `test_scripts/utilities.as`
- `test_scripts/interface.as`
- `test_scripts/using_namespace.as`
- `test_scripts/enum.as`
- `test_scripts/templates.as`
- `test_scripts/game_logic.as`
- `test_scripts/inheritance.as`
- `test_scripts/data_structures.as`
- `test_scripts/nested.as`
- `test_scripts/functions.as`
- `test_scripts/literals.as`
- `test_scripts/class_basic.as`
- `test_scripts/control_flow.as`
- `test_scripts/types.as`
- `test_scripts/performance/large_500.as`
- `test_scripts/performance/xlarge_1000.as`
- `test_scripts/performance/xxlarge_5000.as`

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
