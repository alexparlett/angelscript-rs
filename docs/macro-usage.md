# AngelScript Macros Usage Guide

This document provides comprehensive documentation for all procedural macros in `angelscript-macros`.

## Table of Contents

1. [#[derive(Any)]](#deriveany) - Type registration
2. [#[angelscript_macros::function]](#function) - Function registration
3. [#[angelscript_macros::interface]](#interface) - Interface definition
4. [#[angelscript_macros::funcdef]](#funcdef) - Function pointer types
5. [#[angelscript_macros::global]](#global) - Global constant registration

---

## #[derive(Any)]

Implements the `Any` trait for a type, enabling registration with the AngelScript engine.

### Type Attributes

Apply these to the struct/enum with `#[angelscript(...)]`:

| Attribute | Description |
|-----------|-------------|
| `name = "Name"` | Override the AngelScript type name (default: Rust struct name) |
| `value` | Value type - copied by value, no ref counting |
| `pod` | Plain Old Data - value type safe for raw memory operations |
| `reference` | Reference type - passed by handle, uses ref counting |
| `scoped` | Scoped reference - automatically released at scope exit |
| `nocount` | Single-ref type - no reference counting |
| `template = "<T>"` | Template type with type parameters |
| `specialization_of = "name"` | Base template name for template specialization |
| `specialization_args(T1, T2)` | Type arguments for template specialization |

### Field Attributes

Apply these to struct fields with `#[angelscript(...)]`:

| Attribute | Description |
|-----------|-------------|
| `get` | Generate getter for this property |
| `set` | Generate setter for this property |
| `name = "name"` | Override the property name in AngelScript |

### Examples

#### Basic Value Type

```rust
#[derive(Any)]
#[angelscript(name = "Vec2", pod)]
pub struct Vector2 {
    #[angelscript(get, set)]
    pub x: f32,

    #[angelscript(get, set)]
    pub y: f32,
}
```

#### Reference Type

```rust
#[derive(Any)]
#[angelscript(name = "Player", reference)]
pub struct Player {
    #[angelscript(get)]
    pub id: u64,

    #[angelscript(get, set)]
    pub health: i32,

    #[angelscript(get, set, name = "name")]
    pub player_name: String,
}
```

#### Template Type

```rust
#[derive(Any)]
#[angelscript(name = "array", reference, template = "<T>")]
pub struct ScriptArray;

#[derive(Any)]
#[angelscript(name = "dictionary", reference, template = "<K, V>")]
pub struct ScriptDict;
```

---

## #[angelscript_macros::function]

Marks a function or method for registration with AngelScript. Generates metadata for the registry.

### Function Kind Attributes

| Attribute | Description |
|-----------|-------------|
| (none) | Global function |
| `instance` | Instance method on a type |
| `constructor` | Constructor (returns `Self`) |
| `factory` | Factory function (returns handle to new instance) |
| `destructor` | Destructor |
| `addref` | Reference counting: increment |
| `release` | Reference counting: decrement |
| `list_construct` | List initialization constructor |
| `list_factory` | List initialization factory |
| `template_callback` | Template instantiation callback |

### Modifier Attributes

| Attribute | Description |
|-----------|-------------|
| `const` | Method doesn't modify object state |
| `property` | Virtual property accessor |
| `generic` | Uses generic calling convention (see below) |
| `template` | Template function (deprecated, use `template = "..."`) |
| `template = "<T, U>"` | Template function with named type parameters |
| `copy` | Copy constructor (use with `constructor`) |
| `keep` | Keep original function name callable, use `__meta` suffix |
| `name = "name"` | Override AngelScript function name |
| `property_name = "name"` | Override inferred property name |
| `operator = Operator::X` | Operator overload |

### Operator Values

Use with `operator = Operator::X`:

- Arithmetic: `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Pow`
- Unary: `Neg`, `PreInc`, `PreDec`, `PostInc`, `PostDec`
- Comparison: `Equals`, `Cmp`
- Bitwise: `BitAnd`, `BitOr`, `BitXor`, `BitNot`, `ShiftLeft`, `ShiftRight`
- Compound: `AddAssign`, `SubAssign`, `MulAssign`, `DivAssign`, `ModAssign`, etc.
- Access: `Index`
- Conversion: `Conv`, `ImplConv`
- Other: `Assign`, `Call`, `Cast`

### Parameter Attributes (Non-Generic)

Apply directly on function parameters:

| Attribute | Description |
|-----------|-------------|
| `#[default("value")]` | Default value as string expression |
| `#[template("T")]` | Parameter uses template type parameter |

### Examples

#### Global Function

```rust
#[angelscript_macros::function]
pub fn abs(value: f32) -> f32 {
    value.abs()
}
```

#### Instance Methods

```rust
impl Player {
    #[angelscript_macros::function(constructor)]
    pub fn new(name: String) -> Self {
        Self { name, health: 100 }
    }

    #[angelscript_macros::function(instance, const)]
    pub fn get_health(&self) -> i32 {
        self.health
    }

    #[angelscript_macros::function(instance)]
    pub fn take_damage(&mut self, amount: i32) {
        self.health -= amount;
    }

    #[angelscript_macros::function(instance, const, name = "toString")]
    pub fn to_string(&self) -> String {
        format!("Player({})", self.name)
    }
}
```

#### Default Parameters

```rust
impl ScriptArray {
    #[angelscript_macros::function(instance)]
    pub fn resize(&mut self, #[default("0")] new_size: u32) {
        // ...
    }

    #[angelscript_macros::function(instance, const)]
    pub fn find(&self, value: i32, #[default("0")] start: u32) -> i32 {
        // ...
    }
}
```

#### Operators

```rust
impl Vector2 {
    #[angelscript_macros::function(instance, const, operator = Operator::Add)]
    pub fn op_add(&self, other: &Vector2) -> Vector2 {
        Vector2 { x: self.x + other.x, y: self.y + other.y }
    }

    #[angelscript_macros::function(instance, operator = Operator::AddAssign)]
    pub fn op_add_assign(&mut self, other: &Vector2) {
        self.x += other.x;
        self.y += other.y;
    }

    #[angelscript_macros::function(instance, const, operator = Operator::Index)]
    pub fn op_index(&self, index: u32) -> f32 {
        match index {
            0 => self.x,
            1 => self.y,
            _ => 0.0,
        }
    }

    #[angelscript_macros::function(instance, const, operator = Operator::Neg)]
    pub fn op_neg(&self) -> Vector2 {
        Vector2 { x: -self.x, y: -self.y }
    }
}
```

#### Template Parameter Methods

For methods on template types that use template type parameters:

```rust
impl ScriptArray {
    #[angelscript_macros::function(instance, name = "insertLast")]
    pub fn insert_last(&mut self, #[template("T")] value: AnyRef<'static>) {
        // value's actual type is determined by the template instantiation
        todo!()
    }

    #[angelscript_macros::function(instance, const)]
    pub fn find(&self, #[template("T")] value: AnyRef<'static>) -> i32 {
        todo!()
    }
}

impl ScriptDict {
    #[angelscript_macros::function(instance)]
    pub fn set(
        &mut self,
        #[template("K")] key: AnyRef<'static>,
        #[template("V")] value: AnyRef<'static>,
    ) {
        todo!()
    }
}
```

#### Reference Counting Behaviors

```rust
impl ScriptArray {
    #[angelscript_macros::function(addref)]
    pub fn add_ref(&self) {
        // Increment reference count
    }

    #[angelscript_macros::function(release)]
    pub fn release(&self) -> bool {
        // Decrement reference count, return true if destroyed
        false
    }
}
```

#### Template Callback

For template types, you can register a validation callback that's called when the template is instantiated:

```rust
impl ScriptArray {
    /// Called when array<T> is instantiated to validate T
    #[angelscript_macros::function(template_callback)]
    pub fn validate_template(type_info: &TemplateInstanceInfo) -> bool {
        // Return true if T is a valid type parameter
        // Return false to reject the instantiation
        true
    }
}
```

#### Garbage Collection Behaviors

For types that participate in garbage collection (can contain circular references):

```rust
impl MyGcType {
    #[angelscript_macros::function(gc_getrefcount)]
    pub fn gc_get_ref_count(&self) -> i32 {
        // Return current reference count
        todo!()
    }

    #[angelscript_macros::function(gc_setflag)]
    pub fn gc_set_flag(&mut self) {
        // Set the GC flag
        todo!()
    }

    #[angelscript_macros::function(gc_getflag)]
    pub fn gc_get_flag(&self) -> bool {
        // Get the GC flag
        todo!()
    }

    #[angelscript_macros::function(gc_enumrefs)]
    pub fn gc_enum_refs(&self, /* gc context */) {
        // Enumerate all references this object holds
        todo!()
    }

    #[angelscript_macros::function(gc_releaserefs)]
    pub fn gc_release_refs(&mut self, /* gc context */) {
        // Release all references this object holds
        todo!()
    }
}
```

#### Weak Reference Support

```rust
impl MyType {
    #[angelscript_macros::function(get_weakref_flag)]
    pub fn get_weakref_flag(&self) -> *mut () {
        // Return pointer to weak reference flag
        todo!()
    }
}
```

#### Property Accessors

```rust
impl Player {
    #[angelscript_macros::function(instance, const, property)]
    pub fn get_score(&self) -> i32 {
        self.score
    }

    #[angelscript_macros::function(instance, property)]
    pub fn set_score(&mut self, value: i32) {
        self.score = value;
    }
}
```

---

## Generic Calling Convention

For functions that need to accept any type or have variadic arguments, use the `generic` attribute with `#[param(...)]` and `#[returns(...)]` outer attributes.

**Important:** Generic calling convention functions have a Rust signature of just `fn(_ctx: &CallContext)`. The actual parameter types are described via `#[param(...)]` attributes.

### #[param(...)] Attributes

| Attribute | Description |
|-----------|-------------|
| `type = T` | Specific type for this parameter |
| `variable` | Any type (`?` in AngelScript) |
| `variadic` | Accept multiple arguments (`...`) |
| `in` | Input reference mode |
| `out` | Output reference mode |
| `inout` | Input/output reference mode |
| `default = "expr"` | Default value expression |
| `if_handle_then_const` | When T is handle type, pointed-to object is also const |

### #[returns(...)] Attributes

| Attribute | Description |
|-----------|-------------|
| `type = T` | Explicit return type |
| `variable` | Variable return type (`?`) |
| `ref` | Return by reference |
| `handle` | Return as handle |
| `const` | Const return |

### #[list_pattern(...)] Attributes

For list constructors/factories:

| Attribute | Description |
|-----------|-------------|
| `repeat = T` | Repeating single type: `{T, T, T, ...}` |
| `fixed(T1, T2, T3)` | Fixed sequence: `{T1, T2, T3}` |
| `repeat_tuple(K, V)` | Repeating tuple: `{(K,V), (K,V), ...}` |

### Examples

#### Variadic Print Functions

```rust
use angelscript_core::CallContext;

/// print("format {}", arg1, arg2, ...)
#[angelscript_macros::function(generic, name = "print")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_print(_ctx: &CallContext) {
    // Access args via ctx.get_arg(index)
    todo!()
}

/// println("format {}", arg1, arg2, ...)
#[angelscript_macros::function(generic, name = "println")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_println(_ctx: &CallContext) {
    todo!()
}
```

#### Variable Type Parameters

```rust
/// Function accepting any type
#[angelscript_macros::function(generic)]
#[param(variable, in)]
#[returns(variable)]
pub fn identity(_ctx: &CallContext) {
    // Returns the input unchanged
    todo!()
}
```

#### List Construction

```rust
impl ScriptArray {
    /// array<T> = {1, 2, 3, 4, 5}
    #[angelscript_macros::function(list_factory, generic)]
    #[list_pattern(repeat = i32)]
    pub fn from_list(_ctx: &CallContext) {
        todo!()
    }
}

impl ScriptDict {
    /// dictionary<K,V> = {{"a", 1}, {"b", 2}}
    #[angelscript_macros::function(list_factory, generic)]
    #[list_pattern(repeat_tuple(String, i32))]
    pub fn from_list(_ctx: &CallContext) {
        todo!()
    }
}
```

---

## #[angelscript_macros::interface]

Defines an AngelScript interface from a Rust trait.

### Attributes

| Attribute | Description |
|-----------|-------------|
| `name = "Name"` | Override the AngelScript interface name |

### Method Attributes

Methods within the trait can use:

| Attribute | Description |
|-----------|-------------|
| `#[function(name = "...")]` | Override method name |
| `#[function(const)]` | Explicitly mark as const |

**Note:** Methods with `&self` are automatically marked const. Methods with `&mut self` are non-const.

### Examples

```rust
#[angelscript_macros::interface(name = "IDrawable")]
pub trait Drawable {
    fn draw(&self);  // const inferred from &self
    fn get_bounds(&self) -> Rect;

    #[function(name = "setPosition")]
    fn set_position(&mut self, x: f32, y: f32);  // non-const
}

#[angelscript_macros::interface]
pub trait Updateable {
    fn update(&mut self, delta: f32);

    #[function(const)]  // explicit const override
    fn is_active(&self) -> bool;
}
```

Generated metadata function: `__as_Drawable_interface_meta()` / `__as_Updateable_interface_meta()`

---

## #[angelscript_macros::funcdef]

Creates an AngelScript function pointer type (funcdef) from a Rust type alias.

### Attributes

| Attribute | Description |
|-----------|-------------|
| `name = "Name"` | Override the AngelScript funcdef name |
| `parent = Type` | Parent type for child funcdefs (see Advanced Template Features) |

### Examples

```rust
#[angelscript_macros::funcdef(name = "Callback")]
pub type MyCallback = fn(i32) -> bool;

#[angelscript_macros::funcdef(name = "CompareFunc")]
pub type CompareFunction = fn(&i32, &i32) -> i32;

#[angelscript_macros::funcdef]  // Uses type alias name "EventHandler"
pub type EventHandler = fn(String, i32);
```

Generated metadata function: `__as_MyCallback_funcdef_meta()` etc.

---

## #[angelscript_macros::global]

Registers a Rust constant as an AngelScript global constant. This macro only supports primitive types (integers, floats, bools).

### Attributes

| Attribute | Description |
|-----------|-------------|
| `name = "Name"` | Override the AngelScript constant name (default: Rust const name) |
| `namespace = "ns"` | Namespace path for the constant |

### Supported Types

Only primitive types are supported:
- `bool`
- `i8`, `i16`, `i32`, `i64`
- `u8`, `u16`, `u32`, `u64`
- `f32`, `f64`

### Examples

#### Basic Constant

```rust
#[angelscript_macros::global]
pub const PI: f64 = 3.14159265358979;
```

#### With Namespace

```rust
#[angelscript_macros::global(namespace = "math")]
pub const E: f64 = 2.71828182845905;

#[angelscript_macros::global(namespace = "math")]
pub const TAU: f64 = 6.28318530717959;
```

#### With Name Override

```rust
#[angelscript_macros::global(name = "MAX_INT", namespace = "limits")]
pub const I32_MAX: i32 = i32::MAX;

#[angelscript_macros::global(name = "MIN_INT", namespace = "limits")]
pub const I32_MIN: i32 = i32::MIN;
```

### Generated Code

The macro generates a metadata function:

```rust
pub fn PI__global_meta() -> GlobalMeta {
    GlobalMeta {
        name: "PI",
        namespace: None,
        value: ConstantValue::Double(3.14159265358979),
    }
}
```

### Module Registration

Register constants using `global_meta()`:

```rust
pub fn math_module() -> Module {
    Module::in_namespace(&["math"])
        .global_meta(PI__global_meta)
        .global_meta(E__global_meta)
        .global_meta(TAU__global_meta)
}
```

### Note on Mutable Globals

This macro only supports **immutable constants**. For mutable global properties that scripts can modify, use the explicit `Module::global()` API with `Arc<RwLock<T>>`:

```rust
let score = Arc::new(RwLock::new(0i32));
Module::new()
    .global("g_score", score.clone());  // Mutable int
```

See [Task 25](../claude/tasks/25_global_properties.md) for the full global properties design.

---

## Module Registration

All metadata is collected into `Module` instances for registration:

```rust
use angelscript_registry::Module;

pub fn math_module() -> Module {
    Module::new()
        .function(abs)
        .function(sqrt)
        .function(sin)
        .function(cos)
}

pub fn array_module() -> Module {
    Module::new()
        .ty::<ScriptArray>()
        .function(ScriptArray::add_ref__meta)
        .function(ScriptArray::release__meta)
        .function(ScriptArray::len__meta)
        .function(ScriptArray::insert_last__meta)
        // ...
}
```

### Free Functions vs Methods

- **Free functions** (not in impl block): Use the function name directly: `.function(abs)`
- **Methods** (in impl block): Use the `__meta` suffix: `.function(ScriptArray::len__meta)`

---

## Advanced Template Features

### Template Functions

Template functions are functions that are themselves generic (not just methods on template types). Use `template = "<T>"` or `template = "<T, U>"` to define template parameters:

```rust
/// T Test<T, U>(T t, U u) - A template function with two type parameters
#[angelscript_macros::function(generic, template = "<T, U>")]
#[param(variable, in)]  // T t - first param uses first template type
#[param(variable, in)]  // U u - second param uses second template type
#[returns(variable)]    // returns T
pub fn template_test(_ctx: &CallContext) {
    // Implementation accesses args via ctx.get_arg(index)
    todo!()
}

/// Single template parameter
#[angelscript_macros::function(generic, template = "<T>")]
#[param(variable, in)]
#[returns(variable)]
pub fn identity<T>(_ctx: &CallContext) {
    todo!()
}
```

### Child Funcdefs

Funcdefs can belong to template types (child funcdefs). Use `parent = Type` to associate a funcdef with its parent type:

```rust
/// Global funcdef
#[angelscript_macros::funcdef(name = "Callback")]
pub type Callback = fn(i32) -> bool;

/// Child funcdef of myTemplate<T>
/// In AngelScript: "bool myTemplate<T>::callback(const T &in)"
#[angelscript_macros::funcdef(
    name = "callback",
    parent = ScriptArray  // parent template type
)]
pub type ArrayCallback = fn(&i32) -> bool;
```

### Template Specializations

When you need specialized implementations for specific template instantiations:

```rust
/// Generic template type
#[derive(Any)]
#[angelscript(name = "myTemplate", reference, template = "<T>")]
pub struct MyTemplate<T> {
    _phantom: std::marker::PhantomData<T>,
}

/// Specialization for float: myTemplate<float>
#[derive(Any)]
#[angelscript(
    name = "myTemplate<float>",
    reference,
    specialization_of = "myTemplate",
    specialization_args(f32)
)]
pub struct MyTemplateFloat {
    // Specialized implementation for float
}

/// Multiple type argument specialization: dictionary<string, int>
#[derive(Any)]
#[angelscript(
    name = "dictionary<string, int>",
    reference,
    specialization_of = "dictionary",
    specialization_args(String, i32)
)]
pub struct DictStringInt {
    // Specialized implementation
}
```

### if_handle_then_const

For generic calling convention parameters that should apply `const` transitively when instantiated with a handle type. This attribute is **only available on `#[param(...)]` attributes** for generic functions.

When `if_handle_then_const` is set and the parameter is instantiated with a handle type (e.g., `Obj@`), the pointed-to object is also const:
- `const T&in` with T=`Obj@` becomes `const Obj@ const &in` instead of `Obj@ const &in`

```rust
/// Generic calling convention with if_handle_then_const
#[angelscript_macros::function(generic, instance, const)]
#[param(variable, in, if_handle_then_const)]  // when T is a handle, pointed-to object is const
#[param(type = u32, in, default = "0")]
#[returns(type = i32)]
pub fn find(_ctx: &CallContext) {
    todo!()
}
```

---

## Quick Reference

### Type Registration

```rust
#[derive(Any)]
#[angelscript(name = "...", value|pod|reference|scoped|nocount, template = "<T>")]
pub struct MyType {
    #[angelscript(get, set, name = "...")]
    pub field: i32,
}
```

### Non-Generic Function

```rust
#[angelscript_macros::function(instance|constructor|factory|..., const, operator = Operator::X, name = "...")]
pub fn method(&self, #[default("0")] param: i32, #[template("T")] value: AnyRef<'static>) -> i32 {
    // ...
}
```

### Generic Function

```rust
#[angelscript_macros::function(generic, name = "...")]
#[param(type = T, in|out|inout, variadic, default = "...")]
#[param(variable, in, variadic)]
#[returns(type = T, variable, ref|handle, const)]
#[list_pattern(repeat = T | fixed(T1, T2) | repeat_tuple(K, V))]
pub fn generic_fn(_ctx: &CallContext) {
    // ...
}
```

### Interface

```rust
#[angelscript_macros::interface(name = "...")]
pub trait MyInterface {
    #[function(name = "...", const)]
    fn method(&self) -> i32;
}
```

### Funcdef

```rust
#[angelscript_macros::funcdef(name = "...")]
pub type MyCallback = fn(i32) -> bool;
```

### Global Constant

```rust
#[angelscript_macros::global(name = "...", namespace = "...")]
pub const MY_CONST: f64 = 3.14;
```
