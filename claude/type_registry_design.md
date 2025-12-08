# Unified Type Registry Design

## Overview

Replace the current split architecture (`FfiRegistry` + planned `ScriptRegistry`) with a single unified `SymbolRegistry` that handles FFI, shared script, and local script entities.

**Key decisions:**
- Rename `angelscript-ffi` → `angelscript-registry`
- Adopt rune-style derive macros for ergonomic FFI registration
- Behaviors embedded with types (not separate)
- Template instances stored in registry (same TypeHash across units)
- Fine-grained locking (not whole registry)

## Crate Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     angelscript (main crate)                     │
│                 VM, Context, Engine, public API                  │
└─────────────────────────────────────────────────────────────────┘
        │                    │                    │
        ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│angelscript-     │  │angelscript-     │  │angelscript-     │
│modules          │◄─┤compiler         │  │macros           │
│                 │  │                 │  │                 │
│stdlib: string,  │  │Passes, codegen  │  │#[derive(Any)]   │
│array, dict, etc │  │type checking    │  │#[function]      │
└────────┬────────┘  └─────────────────┘  └────────┬────────┘
         │                    │                    │
         │  ┌─────────────────┼────────────────────┘
         │  │                 │           (proc-macro dep)
         ▼  ▼                 ▼
┌─────────────────┐  ┌─────────────────┐
│angelscript-     │  │angelscript-     │
│registry         │  │parser           │
│                 │  │                 │
│SymbolRegistry,    │  │Lexer, AST       │
│Module           │  │                 │
└────────┬────────┘  └────────┬────────┘
         │                    │
         └────────┬───────────┘
                  ▼
         ┌─────────────────┐
         │ angelscript-    │
         │ core            │
         │                 │
         │ Types, hashes,  │
         │ definitions     │
         └─────────────────┘
```

## Core Types (angelscript-core)

New types to add:
- `TypeEntry`, `TypeSource` - Registry entries with source tracking
- `FunctionEntry`, `FunctionImpl` - Function storage
- `UnitId` - Script compilation unit identifier
- `Op` - Operator enum for macros
- `Any` - Trait for registrable types
- `NativeFn` - Type-erased native function storage (`Box<dyn Any + Send + Sync>`)

## Module API (Namespace-based)

```rust
// Module uses namespace path, not a single name
let mut module = Module::new(&["std", "string"])?;  // Namespace: std::string

// Root module (no namespace)
let mut module = Module::root();

// Register types
module.ty::<ScriptString>()?;
module.function_meta(ScriptString::new)?;

// Install into registry
registry.install(module)?;
```

## Rune-Style Registration API

**Key principle: NO declaration string parsing.** All signatures derived from Rust types or composed from attributes.

### Rust to AngelScript Type Mapping

For normal functions, parameter modes are inferred from Rust types:

| Rust Type | AngelScript | Notes |
|-----------|-------------|-------|
| `T` (Copy) | `T` | Pass by value |
| `T` (non-Copy) | `T` | Pass by value (move) |
| `&T` | `const T &in` | Input reference |
| `&mut T` | `T &inout` | In/out reference (heap types only) |
| `Handle<T>` | `T@` | Object handle |
| `Option<Handle<T>>` | `T@` | Nullable handle |

For `&out` parameters, use explicit attribute since Rust has no equivalent:
```rust
#[angelscript::function]
pub fn get_values(#[angelscript(out)] a: &mut i32, #[angelscript(out)] b: &mut i32) { ... }
```

### Type Registration

```rust
#[derive(Any)]
#[angelscript(name = "MyClass")]
pub struct MyClass {
    // Direct field access as property
    #[angelscript(get, set)]
    pub value: i32,

    // Read-only field
    #[angelscript(get)]
    pub id: u64,

    // Renamed property
    #[angelscript(get, set, name = "count")]
    pub internal_count: i32,
}

impl MyClass {
    #[angelscript::function(constructor)]
    pub fn new(value: i32) -> Self { Self { value } }

    #[angelscript::function(factory)]
    pub fn create(value: i32) -> MyClass { Self { value } }

    #[angelscript::function(instance)]
    pub fn get_value(&self) -> i32 { self.value }

    #[angelscript::function(instance, operator = Op::Add)]
    pub fn add(&self, other: &MyClass) -> MyClass { ... }

    // Generic calling convention for advanced use cases
    #[angelscript::function(instance, generic)]
    pub fn advanced(ctx: &mut CallContext) -> Result<(), Error> {
        // CallContext structure TBD with VM
        Ok(())
    }

    // ========== VIRTUAL PROPERTY ACCESSORS ==========
    // Registered as methods with `property` decorator in AngelScript
    // Script uses `obj.length` syntax, engine calls get_length()/set_length()

    // Option 1: Explicit property attribute (clearest)
    #[angelscript::function(instance, property)]
    pub fn get_length(&self) -> i32 { self.items.len() as i32 }

    #[angelscript::function(instance, property)]
    pub fn get_name(&self) -> &str { &self.name }

    #[angelscript::function(instance, property)]
    pub fn set_name(&mut self, value: String) { self.name = value; }

    // Option 2: Auto-detect from get_/set_ prefix (convenience)
    // If asEP_PROPERTY_ACCESSOR_MODE allows, macro can auto-add `property`
    // Use `name = "..."` to opt-out of property detection

    // Indexed property accessors: get_opIndex/set_opIndex
    #[angelscript::function(instance, property)]
    pub fn get_opIndex(&self, index: i32) -> i32 { self.items[index as usize] }

    #[angelscript::function(instance, property)]
    pub fn set_opIndex(&mut self, index: i32, value: i32) { self.items[index as usize] = value; }

    // NOT a property - just a method named get_raw_data
    #[angelscript::function(instance)]  // No `property` attribute
    pub fn get_raw_data(&self) -> Vec<u8> { ... }
}
```

### Behaviors

```rust
// ===== LIFECYCLE =====
#[angelscript::function(constructor)]           // Constructor (value types)
#[angelscript::function(constructor, copy)]     // Copy constructor
#[angelscript::function(destructor)]            // Destructor
#[angelscript::function(factory)]               // Factory (reference types)
#[angelscript::function(addref)]                // AddRef (ref counting)
#[angelscript::function(release)]               // Release (ref counting)
// Scoped release: uses Rust's Drop trait automatically

// ===== LIST INITIALIZATION =====
// Enables: MyType t = {1, 2, 3};
#[angelscript::function(list_construct, generic)]
#[angelscript::list_pattern(repeat = i32)]      // Variable-length list of same type
pub fn from_list(ctx: &mut CallContext) -> Result<(), Error> { ... }

// Enables: array<int> a = {1, 2, 3};
#[angelscript::function(list_factory, generic)]
#[angelscript::list_pattern(repeat = i32)]
pub fn create_from_list(ctx: &mut CallContext) -> Result<Handle<Self>, Error> { ... }

// Fixed tuple pattern: MyPair p = {1, "hello"};
#[angelscript::function(list_construct, generic)]
#[angelscript::list_pattern(fixed = [i32, String])]
pub fn from_pair(ctx: &mut CallContext) -> Result<(), Error> { ... }

// ===== GC BEHAVIORS (Task 16) =====
#[angelscript::function(gc_getrefcount)]        // Report ref count
#[angelscript::function(gc_setflag)]            // Set GC flag
#[angelscript::function(gc_getflag)]            // Get GC flag
#[angelscript::function(gc_enumrefs)]           // Enumerate refs
#[angelscript::function(gc_releaserefs)]        // Release refs

// ===== WEAK REFERENCE (Task 16) =====
#[angelscript::function(get_weakref_flag)]      // Weak ref flag
```

### Operators (enum-based)

```rust
pub enum Op {
    // Assignment
    Assign, AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
    PowAssign, AndAssign, OrAssign, XorAssign, ShlAssign, ShrAssign, UshrAssign,
    // Binary
    Add, Sub, Mul, Div, Mod, Pow, And, Or, Xor, Shl, Shr, Ushr,
    // Comparison
    Cmp, Equals,
    // Unary
    Neg, Com, PreInc, PreDec, PostInc, PostDec,
    // Index, Call, Cast
    Index, Call,
    // Conversion
    Conv, ImplConv,
    // Handle (Task 18)
    HndlAssign,  // opHndlAssign for ref type
}
```

## Template Types

```rust
#[derive(Any)]
#[angelscript(name = "array", template = "<T>")]
pub struct ScriptArray<T> {
    inner: Vec<T>,
}

impl<T> ScriptArray<T> {
    #[angelscript::function(factory, generic)]
    pub fn create(ctx: &mut CallContext) -> Result<Handle<Self>, Error> {
        // ctx.template_type_arg(0) for T's TypeHash
        todo!()
    }

    #[angelscript::function(instance)]
    pub fn length(&self) -> u64 { self.inner.len() as u64 }
}

#[angelscript::template_callback(for = "array")]
pub fn array_callback(info: &TemplateInstanceInfo) -> TemplateValidation {
    if info.type_arg(0).is_void() {
        return TemplateValidation::Reject("array cannot hold void");
    }
    TemplateValidation::Accept
}
```

### Advanced Template Features (Task 14)

```rust
// if_handle_then_const - attribute on parameter
#[angelscript::function(instance, const)]
pub fn find(&self, #[angelscript(if_handle_then_const)] value: &T) -> i32 { ... }

// Child funcdefs - funcdef belongs to template type
#[angelscript::funcdef(parent = Container<T>)]
pub type Callback = fn(&T) -> bool;

// Template specializations - register as concrete type, not template
#[derive(Any)]
#[angelscript(name = "array", specialization = "int")]
pub struct IntArray { ... }
```

## Template Functions (Task 12)

```rust
// Standalone template function - T inferred from Rust generic
#[angelscript::function(template)]
pub fn max<T: Comparable>(ctx: &mut CallContext) -> Result<(), Error> {
    // ctx.template_type_arg(0) gives T's TypeHash
    // ctx.arg_type_id(n) gives runtime type of arg
    todo!()
}

// Template method on non-template class
impl Container {
    #[angelscript::function(instance, template)]
    pub fn get<T>(ctx: &mut CallContext) -> Result<(), Error> {
        todo!()
    }
}

// Subtype relationships - macro infers from Rust types
#[angelscript::function(template)]
pub fn sort<T>(arr: Handle<ScriptArray<T>>) -> Result<(), Error> {
    // T is inferred from array's type parameter
    todo!()
}
```

## Variadic Functions (Task 13)

**Note:** Variadic functions MUST use generic calling convention (AngelScript requirement).

For generic calling convention, parameter types are specified via attributes (no string parsing):

```rust
// Variadic strings - param attributes define the signature
#[angelscript::function(generic)]
#[angelscript::param(variadic, type = String)]
pub fn print(ctx: &mut CallContext) -> Result<(), Error> {
    for i in 0..ctx.arg_count() {
        let s: &str = ctx.arg(i)?;
        println!("{}", s);
    }
    Ok(())
}

// Variadic integers with return type
#[angelscript::function(generic, returns = i32)]
#[angelscript::param(variadic, type = i32)]
pub fn sum(ctx: &mut CallContext) -> Result<i32, Error> {
    let mut total = 0i32;
    for i in 0..ctx.arg_count() {
        total += ctx.arg::<i32>(i)?;
    }
    Ok(total)
}

// Mixed fixed + variadic params
#[angelscript::function(generic, returns = String)]
#[angelscript::param(type = String)]           // param 0: fixed
#[angelscript::param(variadic, type = String)] // param 1+: variadic
pub fn format(ctx: &mut CallContext) -> Result<String, Error> {
    let fmt: &str = ctx.arg(0)?;
    for i in 1..ctx.arg_count() {
        let s: &str = ctx.arg(i)?;
    }
    Ok(result)
}

// Variadic with any type (?&in)
#[angelscript::function(generic)]
#[angelscript::param(type = i32)]        // param 0: level
#[angelscript::param(variadic, var_in)]  // param 1+: ?&in ...
pub fn log(ctx: &mut CallContext) -> Result<(), Error> {
    let level: i32 = ctx.arg(0)?;
    for i in 1..ctx.arg_count() {
        let type_id = ctx.var_arg_type_id(i - 1)?;
    }
    Ok(())
}
```

**Param attribute options:**
| Attribute | Meaning |
|-----------|---------|
| `type = T` | Parameter is Rust type T |
| `variadic` | This is the variadic parameter (must be last) |
| `var_in` | Parameter is `?&in` (any type input) |
| `var_out` | Parameter is `?&out` (any type output) |
| `ref_in` | Pass by `const &in` |
| `ref_out` | Pass by `&out` |
| `ref_inout` | Pass by `&inout` |

## Enums

```rust
// Rust enum becomes AngelScript enum
#[derive(Any)]
#[angelscript(name = "Color")]
#[repr(i32)]
pub enum Color {
    Red = 0,
    Green = 1,
    Blue = 2,
    // Or auto-increment from Rust discriminants
}

// Registration
module.ty::<Color>()?;
```

## Interfaces

```rust
// Trait becomes AngelScript interface
// Method signatures derived from Rust, but can be customized
#[angelscript::interface]
pub trait Drawable {
    // &self → const method, params inferred from Rust types
    fn draw(&self, x: i32, y: i32);

    // Return type inferred from Rust
    fn get_bounds(&self) -> Rect;

    // Use attributes for special cases
    #[angelscript::method(name = "getSize")]  // Rename
    fn size(&self) -> Size;

    // &mut self → non-const method
    fn reset(&mut self);

    // Output parameter needs explicit attribute
    fn get_position(&self, #[angelscript(out)] x: &mut i32, #[angelscript(out)] y: &mut i32);
}

// Registration
module.interface::<dyn Drawable>()?;
```

Interface methods use same type inference as regular functions - `Any` trait maps Rust types to AngelScript types.

## Funcdefs

```rust
// Type alias becomes funcdef
#[angelscript::funcdef]
pub type Callback = fn(i32) -> bool;

#[angelscript::funcdef(name = "EntityFactory")]
pub type EntityFactoryFn = fn(&str) -> Handle<Entity>;

// Registration
module.funcdef::<Callback>()?;
```

## Default Arguments (Task 19)

```rust
// Default values as Rust attributes
#[angelscript::function]
pub fn greet(name: &str, #[angelscript(default = 1)] times: i32) { ... }

// String default
#[angelscript::function]
pub fn hello(#[angelscript(default = "World")] name: &str) { ... }

// Enum default - use Rust default
#[angelscript::function]
pub fn set_color(#[angelscript(default)] c: Color) { ... }  // Uses Color::default()

// Or explicit enum value
#[angelscript::function]
pub fn set_color(#[angelscript(default = Color::Red)] c: Color) { ... }

// Null default for handles
#[angelscript::function]
pub fn process(#[angelscript(default = null)] obj: Option<Handle<Obj>>) { ... }
```

Storage in registry:
```rust
pub struct SymbolRegistry {
    // ... existing ...
    /// FFI default argument values: (func_hash, param_idx) → FfiExpr
    ffi_defaults: RwLock<FxHashMap<(TypeHash, usize), FfiExpr>>,
}
```

## Generic Handle - ref Type (Task 18)

```rust
#[derive(Any)]
#[angelscript(name = "ref", as_handle)]  // asOBJ_ASHANDLE flag
pub struct ScriptRef {
    handle: Option<ObjectHandle>,
}

impl ScriptRef {
    #[angelscript::function(constructor)]
    pub fn new() -> Self { Self { handle: None } }

    // Constructor from ?&in - uses generic calling convention
    #[angelscript::function(constructor, generic)]
    #[angelscript::param(var_in)]  // ?&in parameter
    pub fn from_any(ctx: &mut CallContext) -> Result<Self, Error> {
        let addr = ctx.var_arg_address(0);
        let type_id = ctx.var_arg_type_id(0);
        todo!()
    }

    // opHndlAssign with ?&in
    #[angelscript::function(instance, operator = Op::HndlAssign, generic)]
    #[angelscript::param(var_in)]
    pub fn assign(ctx: &mut CallContext) -> Result<(), Error> {
        let this: &mut ScriptRef = ctx.this_mut()?;
        let addr = ctx.var_arg_address(0);
        let type_id = ctx.var_arg_type_id(0);
        todo!()
    }

    // opCast with ?&out - allowed per AngelScript spec
    #[angelscript::function(instance, operator = Op::Cast, generic)]
    #[angelscript::param(var_out)]
    pub fn cast(ctx: &mut CallContext) -> Result<(), Error> {
        let this: &ScriptRef = ctx.this()?;
        let out_addr = ctx.var_arg_address(0);
        let out_type_id = ctx.var_arg_type_id(0);
        todo!()
    }

    #[angelscript::function(instance, operator = Op::Equals, generic, returns = bool)]
    #[angelscript::param(var_in)]
    pub fn equals(ctx: &mut CallContext) -> Result<bool, Error> {
        let this: &ScriptRef = ctx.this()?;
        let addr = ctx.var_arg_address(0);
        let type_id = ctx.var_arg_type_id(0);
        todo!()
    }
}
```

**Variable argument types (`?`):**

Per AngelScript docs, `?` parameters provide two values: a reference AND a type ID.

```rust
// In CallContext (when VM is implemented):
impl CallContext {
    /// Get address of ?&in or ?&out parameter
    fn var_arg_address(&self, index: usize) -> *const c_void;

    /// Get type ID of ?&in or ?&out parameter
    fn var_arg_type_id(&self, index: usize) -> TypeId;
}
```

**Supported variants:**
- `?&in` - input reference to any type
- `?&out` - output reference to any type

**NOT supported** (per AngelScript spec):
- `?&inout` - not allowed
- `?` in behaviors/operators (except opCast/opConv)
- `?` in script-side declarations

## Core Design

### Entry Types (Separate Structs)

Each type kind has its own entry struct with exactly the fields it needs:

```rust
pub struct ClassEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub type_kind: TypeKind,
    pub source: TypeSource,
    // Inheritance (use registry to resolve)
    pub base_class: Option<TypeHash>,
    pub interfaces: Vec<TypeHash>,
    // Members (direct access, no lookup needed)
    pub behaviors: TypeBehaviors,
    pub methods: Vec<FunctionEntry>,
    pub properties: Vec<PropertyEntry>,
    pub fields: Vec<FieldEntry>,
    // Template info
    pub template_params: Vec<TypeHash>,  // Non-empty = template definition
    pub template: Option<TypeHash>,       // Template this was instantiated from
    pub type_args: Vec<DataType>,
    // Modifiers
    pub is_final: bool,
    pub is_abstract: bool,
}

pub struct EnumEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub source: TypeSource,
    pub values: Vec<EnumValue>,
}

pub struct InterfaceEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub source: TypeSource,
    pub methods: Vec<MethodSignature>,
    pub base_interfaces: Vec<TypeHash>,
}

pub struct FuncdefEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub source: TypeSource,
    pub params: Vec<DataType>,
    pub return_type: DataType,
}

pub struct PrimitiveEntry {
    pub kind: PrimitiveKind,
    pub type_hash: TypeHash,
}

pub struct TemplateParamEntry {
    pub name: String,
    pub index: usize,
    pub owner: TypeHash,
    pub type_hash: TypeHash,
}
```

### TypeEntry (Unified Enum)

Single enum for unified storage and iteration:

```rust
pub enum TypeEntry {
    Primitive(PrimitiveEntry),
    Class(ClassEntry),
    Enum(EnumEntry),
    Interface(InterfaceEntry),
    Funcdef(FuncdefEntry),
    TemplateParam(TemplateParamEntry),
}

impl TypeEntry {
    // Common accessors
    pub fn type_hash(&self) -> TypeHash {
        match self {
            TypeEntry::Primitive(e) => e.type_hash,
            TypeEntry::Class(e) => e.type_hash,
            TypeEntry::Enum(e) => e.type_hash,
            TypeEntry::Interface(e) => e.type_hash,
            TypeEntry::Funcdef(e) => e.type_hash,
            TypeEntry::TemplateParam(e) => e.type_hash,
        }
    }

    pub fn name(&self) -> &str { ... }
    pub fn qualified_name(&self) -> &str { ... }
    pub fn source(&self) -> Option<&TypeSource> { ... }  // None for Primitive

    // Downcasting
    pub fn as_class(&self) -> Option<&ClassEntry> { ... }
    pub fn as_enum(&self) -> Option<&EnumEntry> { ... }
    pub fn as_interface(&self) -> Option<&InterfaceEntry> { ... }
    pub fn as_funcdef(&self) -> Option<&FuncdefEntry> { ... }

    // Type checks
    pub fn is_class(&self) -> bool { ... }
    pub fn is_enum(&self) -> bool { ... }
    // etc.
}
```

### Supporting Types

```rust
pub enum TypeSource {
    Ffi { rust_type_id: Option<std::any::TypeId> },
    Script { unit_id: UnitId, span: Span },
}

pub struct PropertyEntry {
    pub name: String,
    pub data_type: DataType,
    pub visibility: Visibility,
    pub getter: Option<TypeHash>,   // Virtual property getter
    pub setter: Option<TypeHash>,   // Virtual property setter
}

pub struct FieldEntry {
    pub name: String,
    pub data_type: DataType,
    pub visibility: Visibility,
    pub offset: usize,
}

pub struct EnumValue {
    pub name: String,
    pub value: i64,
}

pub struct MethodSignature {
    pub name: String,
    pub params: Vec<DataType>,
    pub return_type: DataType,
    pub is_const: bool,
}
```

### SymbolRegistry

```rust
pub struct SymbolRegistry {
    // Single map for O(1) type lookup
    types: RwLock<FxHashMap<TypeHash, TypeEntry>>,
    type_by_name: RwLock<FxHashMap<String, TypeHash>>,
    // Global functions (not methods - those live in ClassEntry)
    global_functions: RwLock<FxHashMap<TypeHash, FunctionEntry>>,
    function_overloads: RwLock<FxHashMap<String, Vec<TypeHash>>>,
    // Namespaces
    namespaces: RwLock<FxHashSet<String>>,
    // Template callbacks
    template_callbacks: RwLock<FxHashMap<TypeHash, TemplateCallback>>,
}

impl SymbolRegistry {
    // === Basic Lookup ===
    pub fn get(&self, hash: TypeHash) -> Option<&TypeEntry> { ... }
    pub fn get_by_name(&self, name: &str) -> Option<&TypeEntry> { ... }

    // === Iteration ===
    pub fn types(&self) -> impl Iterator<Item = &TypeEntry> { ... }
    pub fn classes(&self) -> impl Iterator<Item = &ClassEntry> { ... }
    pub fn enums(&self) -> impl Iterator<Item = &EnumEntry> { ... }
    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceEntry> { ... }
    pub fn global_functions(&self) -> impl Iterator<Item = &FunctionEntry> { ... }

    // === Inheritance Helpers (resolve TypeHash chains) ===
    pub fn base_class_chain(&self, hash: TypeHash) -> impl Iterator<Item = &ClassEntry> { ... }
    pub fn all_methods(&self, hash: TypeHash) -> impl Iterator<Item = &FunctionEntry> { ... }
    pub fn all_properties(&self, hash: TypeHash) -> impl Iterator<Item = &PropertyEntry> { ... }

    // === Namespace Helpers ===
    pub fn types_in_namespace(&self, ns: &[&str]) -> impl Iterator<Item = &TypeEntry> { ... }
    pub fn namespaces(&self) -> impl Iterator<Item = &str> { ... }
}
```

### FunctionEntry

```rust
pub struct FunctionEntry {
    pub def: FunctionDef,
    pub implementation: FunctionImpl,
    pub source: FunctionSource,
}

pub struct FunctionDef {
    pub func_hash: TypeHash,
    pub name: String,
    pub namespace: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: DataType,
    pub object_type: Option<TypeHash>,  // None = global function
    pub traits: FunctionTraits,
    pub visibility: Visibility,
    pub template_params: Vec<String>,   // ["T", "U"] for template functions
    pub is_variadic: bool,
}

pub struct FunctionTraits {
    pub is_const: bool,
    pub is_final: bool,
    pub is_override: bool,
    pub is_property: bool,   // Virtual property accessor
    pub is_template: bool,
}

pub enum FunctionImpl {
    Native(NativeFn),  // Box<dyn Any + Send + Sync>
    Script { unit_id: UnitId },
    Abstract,
    External { module: String },
}

pub enum FunctionSource {
    Ffi,
    Script { span: Span },
}
```

### TypeBehaviors

```rust
pub struct TypeBehaviors {
    pub constructors: Vec<TypeHash>,
    pub factories: Vec<TypeHash>,
    pub destructor: Option<TypeHash>,
    pub copy_constructor: Option<TypeHash>,
    pub addref: Option<TypeHash>,
    pub release: Option<TypeHash>,
    pub list_construct: Option<TypeHash>,
    pub list_factory: Option<TypeHash>,
    pub get_weakref_flag: Option<TypeHash>,
    pub template_callback: Option<TypeHash>,
    pub operators: FxHashMap<OperatorBehavior, Vec<TypeHash>>,
    // GC behaviors (Task 16)
    pub gc_getrefcount: Option<TypeHash>,
    pub gc_setflag: Option<TypeHash>,
    pub gc_getflag: Option<TypeHash>,
    pub gc_enumrefs: Option<TypeHash>,
    pub gc_releaserefs: Option<TypeHash>,
}
```

### What Macros Generate

**`#[derive(Any)]` on struct** generates `TypeMeta`:

```rust
#[derive(Any)]
#[angelscript(name = "Player", value)]
pub struct Player {
    #[angelscript(get, set)]
    pub health: i32,
}

// Generates:
impl Player {
    pub fn __as_type_meta() -> ClassMeta {
        ClassMeta {
            name: "Player",
            type_hash: TypeHash::from_name("Player"),
            type_kind: TypeKind::value::<Player>(),
            properties: vec![
                PropertyMeta { name: "health", get: true, set: true, data_type: ... },
            ],
            ..Default::default()
        }
    }
}
```

**`#[angelscript::function]`** generates `FunctionMeta`:

```rust
#[angelscript::function(instance, operator = Op::Add)]
pub fn add(&self, other: &Player) -> Player { ... }

// Generates:
pub fn __as_add_meta() -> FunctionMeta {
    FunctionMeta {
        name: "opAdd",
        func_hash: TypeHash::from_name("Player::opAdd"),
        object_type: Some(<Player as Any>::type_hash()),
        params: vec![...],
        return_type: ...,
        native_fn: Box::new(|...| ...),
        behavior: Some(Behavior::Operator(Op::Add)),
        traits: FunctionTraits { is_const: true, ..Default::default() },
    }
}
```

**Module collects meta and builds entries:**

```rust
impl Module {
    pub fn ty<T: Any>(&mut self) -> Result<(), Error> {
        let meta = T::__as_type_meta();
        self.pending_classes.push(meta);
        Ok(())
    }

    pub fn function_meta(&mut self, f: fn() -> FunctionMeta) {
        self.pending_functions.push(f());
    }

    pub fn install(self, registry: &SymbolRegistry) -> Result<(), Error> {
        // Build ClassEntry from ClassMeta + associated FunctionMetas
        // Register into registry
    }
}
```

## Trait Locations

| Trait/Type | Location | Rationale |
|-----------|----------|-----------|
| `Any` | `angelscript-core` | Only needs `TypeHash` |
| `NativeFn` | `angelscript-core` | Type-erased storage |
| `CallContext` | `angelscript` (main) | TBD with VM |
| `GcContext` | `angelscript` (main) | Task 16 - GC enumeration |
| `WeakRefFlag` | `angelscript` (main) | Task 16 - Weak ref support |

### Any Trait Definition

The `Any` trait maps Rust types to AngelScript types:

```rust
pub trait Any: 'static {
    /// The AngelScript type hash for this type
    fn type_hash() -> TypeHash;

    /// The AngelScript type name (for error messages)
    fn type_name() -> &'static str;
}

// Implemented for primitives
impl Any for i32 {
    fn type_hash() -> TypeHash { primitives::INT32 }
    fn type_name() -> &'static str { "int" }
}

impl Any for String {
    fn type_hash() -> TypeHash { primitives::STRING }
    fn type_name() -> &'static str { "string" }
}

// Derived for user types
#[derive(Any)]
#[angelscript(name = "MyClass")]
pub struct MyClass { ... }
// Generates: TypeHash::from_name("MyClass")
```

When macro sees `#[angelscript::param(type = String)]`:
```rust
// Macro generates:
let param_type = <String as Any>::type_hash();
```

For generic calling convention, types in attributes must implement `Any`.

## Feature Comparison: Current Builders vs Macros

| Current Builder | Macro Equivalent | Status |
|----------------|------------------|--------|
| **Type Kinds** | | |
| `.value_type()` | `#[angelscript(value)]` | ✓ |
| `.pod_type()` | `#[angelscript(pod)]` | ✓ |
| `.reference_type()` | `#[angelscript(reference)]` | ✓ |
| `.scoped_type()` | `#[angelscript(scoped)]` | ✓ |
| `.single_ref_type()` | `#[angelscript(nocount)]` | ✓ |
| **Behaviors** | | |
| `.constructor(decl, f)` | `#[angelscript::function(constructor)]` | ✓ |
| `.factory(decl, f)` | `#[angelscript::function(factory)]` | ✓ |
| `.factory_raw(decl, f)` | `#[angelscript::function(factory, generic)]` | ✓ |
| `.addref(f)` | `#[angelscript::function(addref)]` | ✓ |
| `.release(f)` | `#[angelscript::function(release)]` | ✓ |
| `.destructor(f)` | `#[angelscript::function(destructor)]` | ✓ |
| `.list_construct(pattern, f)` | `#[angelscript::function(list_construct, pattern = ...)]` | ✓ |
| `.list_factory(pattern, f)` | `#[angelscript::function(list_factory, pattern = ...)]` | ✓ |
| `.set_get_weakref_flag(f)` | `#[angelscript::function(get_weakref_flag)]` | ✓ |
| **Members** | | |
| `.method(decl, f)` | `#[angelscript::function(instance)]` | ✓ |
| `.method_mut(decl, f)` | `#[angelscript::function(instance)]` on `&mut self` | ✓ |
| `.method_raw(decl, f)` | `#[angelscript::function(instance, generic)]` | ✓ |
| `.property_get(decl, getter)` | `#[angelscript(get)]` on field OR `#[angelscript::function(instance, property)]` on `get_X()` | ✓ |
| `.property(decl, getter, setter)` | `#[angelscript(get, set)]` on field OR `property` on `get_X()`+`set_X()` methods | ✓ |
| `.operator(decl, f)` | `#[angelscript::function(operator = Op::Add)]` | ✓ |
| `.operator_raw(decl, f)` | `#[angelscript::function(operator = Op::Add, generic)]` | ✓ |
| **Templates** | | |
| `.template_callback(f)` | `#[angelscript::template_callback]` | ✓ |
| **Enums** | | |
| `.value(name, val)` | `#[derive(Any)]` on Rust enum | ✓ |
| `.auto_value(name)` | Rust enum discriminants | ✓ |
| **Interfaces** | | |
| `register_interface(name).method(decl)` | `#[angelscript::interface]` trait | ✓ |
| **Funcdefs** | | |
| `register_funcdef(decl)` | `#[angelscript::funcdef]` on type alias | ✓ |
| **Globals** | | |
| `register_fn(decl, f)` | `#[angelscript::function]` (no `instance`) | ✓ |
| `register_fn_raw(decl, f)` | `#[angelscript::function(generic)]` | ✓ |
| `register_global_property(decl, ref)` | `module.property::<T>(name, ref)?` | Manual |

### Additional Notes

- **Copy constructor**: `#[angelscript::function(constructor, copy)]`
- **Scoped types**: Use Rust's `Drop` trait - macro registers it automatically
- **Virtual properties**: Use `#[angelscript::function(instance, property)]` on `get_X()`/`set_X()` methods
  - Registers method with `property` decorator in AngelScript signature
  - Enables `obj.x` syntax in scripts

## Script Types → Unified Registry

The same `TypeEntry` and `FunctionEntry` structures work for both FFI and script-defined types.

### Source Tracking

```rust
pub enum TypeSource {
    /// FFI type from Rust
    Ffi { rust_type_id: Option<std::any::TypeId> },
    /// Script-defined type
    Script { unit_id: UnitId, span: Span },
    /// Primitive built-in
    Primitive,
}

pub enum FunctionImpl {
    /// FFI: Rust native function
    Native(NativeFn),
    /// Script: Bytecode (TBD with VM)
    Script { unit_id: UnitId },
    /// Abstract method (interface/virtual)
    Abstract,
    /// External import
    External { module: String },
}
```

### AST → Registry Mapping

| AST Node | Registry Entry |
|----------|---------------|
| `ClassDecl` | `TypeEntry` with `TypeSource::Script`, behaviors from methods |
| `FunctionDecl` | `FunctionEntry` with `FunctionImpl::Script` |
| `InterfaceDecl` | `TypeEntry` (interface kind) |
| `EnumDecl` | `TypeEntry` (enum kind) with values |
| `FuncdefDecl` | `TypeEntry` (funcdef kind) |
| `VirtualPropertyDecl` | `get_X`/`set_X` `FunctionEntry` pairs |
| `FieldDecl` | Property in `TypeEntry` (offset-based) |

### Example: Script Class to Registry

```angelscript
class Player : Entity {
    private int health = 100;

    Player(int hp) { health = hp; }
    void takeDamage(int amount) { health -= amount; }
    int get_health() const property { return health; }
}
```

Becomes:
```rust
TypeEntry::Class(ClassEntry {
    name: "Player".into(),
    qualified_name: "Player".into(),
    type_hash: TypeHash::from_name("Player"),
    type_kind: TypeKind::script_object(),
    source: TypeSource::Script { unit_id, span },
    base_class: Some(entity_hash),
    interfaces: vec![],
    behaviors: TypeBehaviors {
        constructors: vec![player_ctor_hash],
        ..Default::default()
    },
    methods: vec![
        // Constructor
        FunctionEntry {
            def: FunctionDef { name: "Player".into(), params: vec![int_param], .. },
            implementation: FunctionImpl::Script { unit_id },
            source: FunctionSource::Script { span: ctor_span },
        },
        // Method
        FunctionEntry {
            def: FunctionDef { name: "takeDamage".into(), object_type: Some(player_hash), .. },
            implementation: FunctionImpl::Script { unit_id },
            source: FunctionSource::Script { span: method_span },
        },
        // Property accessor (is_property flag set)
        FunctionEntry {
            def: FunctionDef {
                name: "get_health".into(),
                traits: FunctionTraits { is_property: true, is_const: true, ..Default::default() },
                ..
            },
            implementation: FunctionImpl::Script { unit_id },
            source: FunctionSource::Script { span: prop_span },
        },
    ],
    properties: vec![
        PropertyEntry {
            name: "health".into(),
            data_type: DataType::int32(),
            visibility: Visibility::Public,
            getter: Some(get_health_hash),
            setter: None,  // read-only
        },
    ],
    fields: vec![
        FieldEntry {
            name: "health".into(),
            data_type: DataType::int32(),
            visibility: Visibility::Private,
            offset: 0,
        },
    ],
    ..Default::default()
})
```

The caller doesn't need to know if it's FFI or script - the unified registry abstracts this.

## Implementation Phases

### Phase 1: Core Types (angelscript-core)
- `src/entries.rs` - ClassEntry, EnumEntry, InterfaceEntry, FuncdefEntry, etc.
- `src/type_entry.rs` - TypeEntry enum wrapping all entry types
- `src/function_entry.rs` - FunctionEntry, FunctionImpl, FunctionSource
- `src/ids.rs` - UnitId
- `src/op.rs` - Op enum (for macro attribute)
- `src/any.rs` - Any trait
- Update `function_def.rs` - Add template_params, is_variadic

### Phase 2: Create angelscript-registry
- `src/registry.rs` - SymbolRegistry
- `src/module.rs` - Module builder with namespace support

### Phase 3: Create angelscript-macros
- `#[derive(Any)]`
- `#[angelscript::function]`
- `#[angelscript::param]` for generic calling convention
- `#[angelscript::interface]`
- `#[angelscript::funcdef]`
- `#[angelscript::template_callback]`
- `#[angelscript::list_pattern]` for list behaviors

### Phase 4: Update Consumers
- Main crate: Use SymbolRegistry
- Compiler: Update to new registry

### Phase 5: Migrate stdlib
- Update to macro-based registration

## Crates to Delete/Keep

**Delete/Merge:**
- `crates/angelscript-ffi/` → Rename to `angelscript-registry`
- `crates/angelscript-module/` → Merge into registry

**Keep:**
- `crates/angelscript-modules/` - Update to macros
- `crates/angelscript-core/` - Add new types
- `crates/angelscript-parser/` - No changes
- `crates/angelscript-compiler/` - Update to SymbolRegistry

## VM Integration (TBD)

Deferred until VM is implemented:
- `CallContext` structure
- `GcContext` for enumrefs/releaserefs
- `WeakRefFlag` implementation
- Stack/register representation
- Type conversion traits

The `NativeFn = Box<dyn Any + Send + Sync>` allows storing native functions without committing to the calling convention. The VM will define the actual callable trait and downcast when calling.
