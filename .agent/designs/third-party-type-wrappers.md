# Design: Third-Party Type Wrapper APIs

## Problem Statement

When integrating AngelScript with third-party libraries like Bevy, users need to expose external types (e.g., `bevy_math::Vec3`) to scripts. Currently this requires:

1. Creating a newtype wrapper struct
2. Implementing `#[derive(Any)]` on the wrapper
3. Manually writing forwarding methods for every function to expose
4. Registering each method individually

This is tedious and error-prone for types with many methods (Bevy's `Vec3` has 50+ methods).

## Current Architecture Summary

**Key Components:**
- `Any` trait: Provides `type_hash()` and `type_name()` for type identity
- `HasClassMeta` trait: Provides `ClassMeta` for type registration
- `FunctionMeta`: Describes function signature, behavior, parameters
- `NativeFn`: Type-erased callable wrapping a `Fn(&mut CallContext) -> Result<(), NativeError>`
- `CallContext`: Provides access to argument slots (`VmSlot`) and return slot
- `VmSlot`: Enum holding `Int(i64)`, `Float(f64)`, `Object(ObjectHandle)`, etc.

**Registration Flow:**
```
#[derive(Any)] → HasClassMeta::__as_type_meta() → ClassMeta
#[angelscript::function] → HasFunctionMeta::__as_fn_meta() → FunctionMeta
Module::new().ty::<T>().function(f) → collects metadata
Context::install(module) → converts to ClassEntry/FunctionEntry → SymbolRegistry
```

---

## Option 1: Declarative Wrapper Macro (`wrap_type!`)

### Overview

A declarative macro that generates newtype wrapper + all forwarding code from a concise DSL.

### Syntax

```rust
angelscript::wrap_type! {
    /// Optional doc comment
    #[angelscript(name = "Vec3", pod)]
    Vec3 = bevy_math::Vec3 {
        // Constructor - generates factory or constructor behavior
        constructor(x: f32, y: f32, z: f32) => Vec3::new(x, y, z),

        // Copy constructor (for value types)
        copy => |v| v.clone(),

        // Methods to forward - just list names, types inferred
        methods: [
            length,           // &self -> f32
            normalize,        // &self -> Self
            dot(other),       // &self, &Self -> f32
            cross(other),     // &self, &Self -> Self
            lerp(other, t),   // &self, &Self, f32 -> Self
        ],

        // Const methods (explicit)
        const_methods: [length, normalize, dot, cross],

        // Mutable methods
        mut_methods: [normalize_mut],

        // Operators - maps to AngelScript operators
        operators: {
            Add => |a, b| *a + *b,           // opAdd
            Sub => |a, b| *a - *b,           // opSub
            Neg => |a| -*a,                  // opNeg
            Mul<f32> => |a, s| *a * s,       // opMul(float)
            Div<f32> => |a, s| *a / s,       // opDiv(float)
            MulAssign<f32> => |a, s| *a *= s, // opMulAssign
            Index<usize> => |a, i| a[i],     // opIndex
            Equals => |a, b| *a == *b,       // opEquals
            Compare => |a, b| a.partial_cmp(b), // opCmp
        },

        // Properties - generates get_x/set_x methods
        properties: {
            x: f32 [get, set],
            y: f32 [get, set],
            z: f32 [get, set],
        },

        // Read-only computed properties
        computed: {
            length_squared: f32 => |v| v.length_squared(),
        },

        // Static methods (no self)
        statics: {
            ZERO => Vec3::ZERO,
            ONE => Vec3::ONE,
            unit_x() => Vec3::X,
            splat(v: f32) => Vec3::splat(v),
        },
    }
}
```

### Generated Code

```rust
// Newtype wrapper
#[repr(transparent)]
pub struct Vec3(pub bevy_math::Vec3);

// Implement Any trait
impl ::angelscript_core::Any for Vec3 {
    fn type_hash() -> ::angelscript_core::TypeHash {
        ::angelscript_core::TypeHash::from_name("Vec3")
    }
    fn type_name() -> &'static str { "Vec3" }
}

// Implement HasClassMeta
impl ::angelscript_registry::HasClassMeta for Vec3 {
    fn __as_type_meta() -> ::angelscript_core::ClassMeta {
        ::angelscript_core::ClassMeta {
            name: "Vec3",
            type_hash: <Vec3 as ::angelscript_core::Any>::type_hash(),
            type_kind: ::angelscript_core::TypeKind::pod::<Vec3>(),
            properties: vec![
                ::angelscript_core::PropertyMeta { name: "x", get: true, set: true, type_hash: <f32>::type_hash() },
                ::angelscript_core::PropertyMeta { name: "y", get: true, set: true, type_hash: <f32>::type_hash() },
                ::angelscript_core::PropertyMeta { name: "z", get: true, set: true, type_hash: <f32>::type_hash() },
            ],
            template_params: vec![],
            specialization_of: None,
            specialization_args: vec![],
        }
    }
}

// Implement Deref for ergonomic access
impl std::ops::Deref for Vec3 {
    type Target = bevy_math::Vec3;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for Vec3 {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

// Implement From for conversions
impl From<bevy_math::Vec3> for Vec3 {
    fn from(v: bevy_math::Vec3) -> Self { Self(v) }
}

impl From<Vec3> for bevy_math::Vec3 {
    fn from(v: Vec3) -> Self { v.0 }
}

// Generated methods with metadata
impl Vec3 {
    #[angelscript::function(constructor)]
    pub fn __new(x: f32, y: f32, z: f32) -> Self {
        Self(bevy_math::Vec3::new(x, y, z))
    }

    #[angelscript::function(instance, const)]
    pub fn length(&self) -> f32 {
        self.0.length()
    }

    #[angelscript::function(instance, const)]
    pub fn normalize(&self) -> Self {
        Self(self.0.normalize())
    }

    #[angelscript::function(instance, const)]
    pub fn dot(&self, other: &Self) -> f32 {
        self.0.dot(other.0)
    }

    // Operator methods
    #[angelscript::function(instance, operator = Operator::Add)]
    pub fn __op_add(&self, other: &Self) -> Self {
        Self(self.0 + other.0)
    }

    #[angelscript::function(instance, operator = Operator::Mul)]
    pub fn __op_mul_f32(&self, scalar: f32) -> Self {
        Self(self.0 * scalar)
    }

    // Property accessors
    #[angelscript::function(instance, const, property)]
    pub fn get_x(&self) -> f32 { self.0.x }

    #[angelscript::function(instance, property)]
    pub fn set_x(&mut self, value: f32) { self.0.x = value; }

    // ... etc for all declared items
}

// Module helper function
impl Vec3 {
    pub fn register_module() -> ::angelscript_registry::Module {
        ::angelscript_registry::Module::new()
            .ty::<Self>()
            .function(Self::__new__meta)
            .function(Self::length__meta)
            .function(Self::normalize__meta)
            .function(Self::dot__meta)
            .function(Self::__op_add__meta)
            .function(Self::__op_mul_f32__meta)
            .function(Self::get_x__meta)
            .function(Self::set_x__meta)
            // ... all methods
    }
}
```

### Usage

```rust
// Register the wrapped type
let module = Vec3::register_module();
context.install(module)?;

// Or combine with other types
let math_module = Module::in_namespace(&["math"])
    .merge(Vec3::register_module())
    .merge(Quat::register_module())
    .merge(Mat4::register_module());
```

### Implementation Complexity

**Pros:**
- Most ergonomic for users
- Single source of truth
- Compile-time type checking
- Zero runtime overhead

**Cons:**
- Complex macro implementation (~500-800 lines of `macro_rules!`)
- Limited to what macros can express
- Error messages may be confusing
- Can't introspect methods automatically (must list them)

### Implementation Files

1. `crates/angelscript-macros/src/wrap_type.rs` - New macro implementation
2. Update `crates/angelscript-macros/src/lib.rs` - Export macro

---

## Option 2: Builder-Style Runtime Registration

### Overview

A fluent builder API that constructs type and function metadata at runtime without proc macros.

### API Design

```rust
use angelscript_registry::{WrapBuilder, WrapType};

// Step 1: Create wrapper type (still needed for type identity)
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vec3(pub bevy_math::Vec3);

// Minimal Any implementation (could be a simple derive)
impl angelscript_core::Any for Vec3 {
    fn type_hash() -> TypeHash { TypeHash::from_name("Vec3") }
    fn type_name() -> &'static str { "Vec3" }
}

// Step 2: Build registration using fluent API
let module = WrapBuilder::<Vec3>::new("Vec3")
    .pod()  // or .value() or .reference()

    // Constructor
    .constructor(|x: f32, y: f32, z: f32| Vec3(bevy_math::Vec3::new(x, y, z)))

    // Methods - closure receives &self or &mut self
    .method("length", |v: &Vec3| v.0.length())
    .method("normalize", |v: &Vec3| Vec3(v.0.normalize()))
    .method_mut("normalize_mut", |v: &mut Vec3| { v.0 = v.0.normalize(); })

    // Const qualifier
    .method_const("dot", |v: &Vec3, other: &Vec3| v.0.dot(other.0))

    // Operators
    .op_add(|a: &Vec3, b: &Vec3| Vec3(a.0 + b.0))
    .op_sub(|a: &Vec3, b: &Vec3| Vec3(a.0 - b.0))
    .op_neg(|a: &Vec3| Vec3(-a.0))
    .op_mul(|a: &Vec3, s: f32| Vec3(a.0 * s))
    .op_equals(|a: &Vec3, b: &Vec3| a.0 == b.0)

    // Properties
    .property("x",
        |v: &Vec3| v.0.x,           // getter
        |v: &mut Vec3, x: f32| v.0.x = x  // setter
    )
    .property_get("length_squared", |v: &Vec3| v.0.length_squared())

    // Static values (global properties in type's namespace)
    .static_value("ZERO", Vec3(bevy_math::Vec3::ZERO))
    .static_value("ONE", Vec3(bevy_math::Vec3::ONE))

    // Static methods
    .static_method("splat", |v: f32| Vec3(bevy_math::Vec3::splat(v)))

    // Build into a Module
    .build();

context.install(module)?;
```

### Core Traits and Types

```rust
// crates/angelscript-registry/src/wrap_builder.rs

use std::marker::PhantomData;

/// Builder for wrapping external types.
pub struct WrapBuilder<T> {
    name: String,
    type_kind: TypeKind,
    class_meta: ClassMeta,
    functions: Vec<FunctionMeta>,
    native_fns: Vec<(TypeHash, NativeFn)>,
    globals: Vec<GlobalPropertyEntry>,
    _phantom: PhantomData<T>,
}

impl<T: Any + 'static> WrapBuilder<T> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_kind: TypeKind::reference(),
            class_meta: ClassMeta { ... },
            functions: Vec::new(),
            native_fns: Vec::new(),
            globals: Vec::new(),
            _phantom: PhantomData,
        }
    }

    // Type kind setters
    pub fn value(mut self) -> Self {
        self.type_kind = TypeKind::value::<T>();
        self
    }

    pub fn pod(mut self) -> Self {
        self.type_kind = TypeKind::pod::<T>();
        self
    }

    pub fn reference(mut self) -> Self {
        self.type_kind = TypeKind::reference();
        self
    }
}
```

### Method Registration Implementation

```rust
impl<T: Any + Send + Sync + 'static> WrapBuilder<T> {
    /// Register a constructor.
    pub fn constructor<Args, F>(mut self, f: F) -> Self
    where
        F: IntoNativeConstructor<T, Args>,
    {
        let (meta, native) = f.into_constructor_parts::<T>(&self.name);
        self.functions.push(meta);
        self.native_fns.push((meta.compute_hash(), native));
        self
    }

    /// Register an instance method.
    pub fn method<Args, R, F>(mut self, name: &str, f: F) -> Self
    where
        F: IntoNativeMethod<T, Args, R>,
    {
        let (meta, native) = f.into_method_parts::<T>(name, false);
        self.functions.push(meta);
        self.native_fns.push((meta.compute_hash(), native));
        self
    }

    /// Register a const instance method.
    pub fn method_const<Args, R, F>(mut self, name: &str, f: F) -> Self
    where
        F: IntoNativeMethod<T, Args, R>,
    {
        let (meta, native) = f.into_method_parts::<T>(name, true);
        self.functions.push(meta);
        self.native_fns.push((meta.compute_hash(), native));
        self
    }
}
```

### Trait Implementations for Closures

```rust
// Trait for converting closures to native methods
pub trait IntoNativeMethod<T, Args, R> {
    fn into_method_parts(self, name: &str, is_const: bool) -> (FunctionMeta, NativeFn);
}

// Implementation for fn(&T) -> R
impl<T, R, F> IntoNativeMethod<T, (), R> for F
where
    T: Any + Send + Sync + 'static,
    R: IntoVmSlot + 'static,
    F: Fn(&T) -> R + Send + Sync + 'static,
{
    fn into_method_parts(self, name: &str, is_const: bool) -> (FunctionMeta, NativeFn) {
        let meta = FunctionMeta {
            name,
            is_method: true,
            is_const,
            associated_type: Some(T::type_hash()),
            params: vec![],
            return_meta: ReturnMeta::from_type::<R>(),
            ..Default::default()
        };

        let native = NativeFn::new(move |ctx: &mut CallContext| {
            let this = ctx.arg_slot(0)?.extract::<T>(ctx.heap())?;
            let result = (self)(this);
            ctx.set_return_slot(result.into_vm_slot());
            Ok(())
        });

        (meta, native)
    }
}

// Implementation for fn(&T, A1) -> R
impl<T, A1, R, F> IntoNativeMethod<T, (A1,), R> for F
where
    T: Any + Send + Sync + 'static,
    A1: FromVmSlot + 'static,
    R: IntoVmSlot + 'static,
    F: Fn(&T, A1) -> R + Send + Sync + 'static,
{
    fn into_method_parts(self, name: &str, is_const: bool) -> (FunctionMeta, NativeFn) {
        let meta = FunctionMeta {
            name,
            is_method: true,
            is_const,
            associated_type: Some(T::type_hash()),
            params: vec![ParamMeta::from_type::<A1>("arg0")],
            return_meta: ReturnMeta::from_type::<R>(),
            ..Default::default()
        };

        let native = NativeFn::new(move |ctx: &mut CallContext| {
            let this = ctx.arg_slot(0)?.extract::<T>(ctx.heap())?;
            let a1 = ctx.arg_slot(1)?.extract::<A1>(ctx.heap())?;
            let result = (self)(this, a1);
            ctx.set_return_slot(result.into_vm_slot());
            Ok(())
        });

        (meta, native)
    }
}

// ... implementations for more arities (up to 8-12 args)
```

### Conversion Traits

```rust
/// Convert Rust value to VmSlot
pub trait IntoVmSlot {
    fn into_vm_slot(self) -> VmSlot;
}

impl IntoVmSlot for i32 {
    fn into_vm_slot(self) -> VmSlot { VmSlot::Int(self as i64) }
}

impl IntoVmSlot for f32 {
    fn into_vm_slot(self) -> VmSlot { VmSlot::Float(self as f64) }
}

impl IntoVmSlot for f64 {
    fn into_vm_slot(self) -> VmSlot { VmSlot::Float(self) }
}

impl IntoVmSlot for bool {
    fn into_vm_slot(self) -> VmSlot { VmSlot::Bool(self) }
}

impl IntoVmSlot for String {
    fn into_vm_slot(self) -> VmSlot { VmSlot::String(self) }
}

impl<T: Any + Send + Sync + 'static> IntoVmSlot for T {
    fn into_vm_slot(self) -> VmSlot {
        VmSlot::Native(Box::new(self))
    }
}

/// Extract Rust value from VmSlot
pub trait FromVmSlot: Sized {
    fn from_vm_slot(slot: &VmSlot, heap: &ObjectHeap) -> Result<Self, NativeError>;
}

impl FromVmSlot for i32 {
    fn from_vm_slot(slot: &VmSlot, _: &ObjectHeap) -> Result<Self, NativeError> {
        match slot {
            VmSlot::Int(v) => Ok(*v as i32),
            _ => Err(NativeError::type_mismatch("int", slot.type_name())),
        }
    }
}

// ... etc for all types
```

### Build Method

```rust
impl<T: Any + Send + Sync + 'static> WrapBuilder<T> {
    pub fn build(self) -> Module {
        let mut module = Module::new();

        // Add the class
        module.classes.push(self.class_meta);

        // Add all functions
        module.functions.extend(self.functions);

        // Add globals
        module.globals.extend(self.globals);

        // Store native function pointers for Context to pick up
        // (This needs integration with Context::install)
        for (hash, native_fn) in self.native_fns {
            module.native_fns.insert(hash, native_fn);
        }

        module
    }
}
```

### Implementation Complexity

**Pros:**
- No new proc macros needed
- Very flexible - any closure works
- Clear, explicit registration
- Easy to understand what's happening
- Works with any Rust type

**Cons:**
- More verbose than macro approach
- Still need newtype + Any impl
- Trait implementations for all arities (macro can generate these)
- Runtime metadata construction (small overhead)

### Implementation Files

1. `crates/angelscript-registry/src/wrap_builder.rs` - Main builder
2. `crates/angelscript-core/src/conversion.rs` - `IntoVmSlot`/`FromVmSlot` traits
3. `crates/angelscript-registry/src/into_native.rs` - `IntoNativeMethod` traits
4. Update `Module` to store native function pointers

---

## Option 3: External Config + Build Script

### Overview

Define bindings in a configuration file, generate Rust code at build time.

### Configuration Format (TOML)

```toml
# angelscript-bindings.toml

[settings]
output_dir = "src/generated"
crate_name = "bevy_angelscript"

[[types]]
name = "Vec3"
rust_type = "bevy_math::Vec3"
kind = "pod"

[types.constructor]
args = ["x: f32", "y: f32", "z: f32"]
expr = "Vec3::new(x, y, z)"

[types.methods]
# Simple forwarding - just method names
forward = ["length", "normalize", "dot", "cross", "lerp"]

# Custom implementations
[types.methods.custom]
distance = { args = ["other: &Self"], ret = "f32", expr = "self.distance(*other)" }

[types.properties]
x = { type = "f32", get = true, set = true }
y = { type = "f32", get = true, set = true }
z = { type = "f32", get = true, set = true }
length_squared = { type = "f32", get = true, expr = "self.length_squared()" }

[types.operators]
Add = "self + other"
Sub = "self - other"
Neg = "-self"
"Mul<f32>" = "self * other"
"Div<f32>" = "self / other"
Equals = "self == other"

[types.statics]
ZERO = "Vec3::ZERO"
ONE = "Vec3::ONE"
X = "Vec3::X"
Y = "Vec3::Y"
Z = "Vec3::Z"

[[types]]
name = "Quat"
rust_type = "bevy_math::Quat"
kind = "pod"
# ... similar structure
```

### Alternative: Rust DSL File

```rust
// bindings.rs (not compiled, parsed by build script)
wrap! {
    Vec3 = bevy_math::Vec3 [pod] {
        new(x: f32, y: f32, z: f32);

        fn length(&self) -> f32;
        fn normalize(&self) -> Self;
        fn dot(&self, other: &Self) -> f32;

        prop x: f32 { get; set; }
        prop y: f32 { get; set; }
        prop z: f32 { get; set; }

        op Add = self + other;
        op Sub = self - other;
        op Mul<f32> = self * other;

        static ZERO = Vec3::ZERO;
        static ONE = Vec3::ONE;
    }
}
```

### Build Script

```rust
// build.rs
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=angelscript-bindings.toml");

    let config = fs::read_to_string("angelscript-bindings.toml")
        .expect("Failed to read bindings config");

    let bindings: BindingsConfig = toml::from_str(&config)
        .expect("Failed to parse bindings config");

    let output_dir = Path::new(&bindings.settings.output_dir);
    fs::create_dir_all(output_dir).unwrap();

    for type_def in &bindings.types {
        let code = generate_wrapper(type_def);
        let filename = format!("{}.rs", type_def.name.to_lowercase());
        fs::write(output_dir.join(&filename), code).unwrap();
    }

    // Generate mod.rs
    let mod_contents = bindings.types.iter()
        .map(|t| format!("mod {};", t.name.to_lowercase()))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(output_dir.join("mod.rs"), mod_contents).unwrap();
}

fn generate_wrapper(type_def: &TypeDef) -> String {
    let mut code = String::new();

    // Generate wrapper struct
    writeln!(code, "#[repr(transparent)]");
    writeln!(code, "#[derive(Clone, Copy)]");
    writeln!(code, "pub struct {}(pub {});", type_def.name, type_def.rust_type);

    // Generate Any impl
    writeln!(code, "impl ::angelscript_core::Any for {} {{", type_def.name);
    writeln!(code, "    fn type_hash() -> TypeHash {{ TypeHash::from_name(\"{}\") }}", type_def.name);
    writeln!(code, "    fn type_name() -> &'static str {{ \"{}\" }}", type_def.name);
    writeln!(code, "}}");

    // Generate methods
    writeln!(code, "impl {} {{", type_def.name);

    // Constructor
    if let Some(ctor) = &type_def.constructor {
        writeln!(code, "    #[angelscript::function(constructor)]");
        writeln!(code, "    pub fn new({}) -> Self {{", ctor.args.join(", "));
        writeln!(code, "        Self({})", ctor.expr);
        writeln!(code, "    }}");
    }

    // Forward methods
    for method in &type_def.methods.forward {
        // Analyze signature via syn or assume simple patterns
        writeln!(code, "    #[angelscript::function(instance, const)]");
        writeln!(code, "    pub fn {}(&self) -> /* inferred */ {{", method);
        writeln!(code, "        self.0.{}()", method);
        writeln!(code, "    }}");
    }

    // Properties
    for (name, prop) in &type_def.properties {
        if prop.get {
            writeln!(code, "    #[angelscript::function(instance, const, property)]");
            writeln!(code, "    pub fn get_{}(&self) -> {} {{", name, prop.type_name);
            if let Some(expr) = &prop.expr {
                writeln!(code, "        {}", expr);
            } else {
                writeln!(code, "        self.0.{}", name);
            }
            writeln!(code, "    }}");
        }
        if prop.set {
            writeln!(code, "    #[angelscript::function(instance, property)]");
            writeln!(code, "    pub fn set_{}(&mut self, value: {}) {{", name, prop.type_name);
            writeln!(code, "        self.0.{} = value;", name);
            writeln!(code, "    }}");
        }
    }

    // Operators
    for (op, expr) in &type_def.operators {
        let op_attr = match op.as_str() {
            "Add" => "Operator::Add",
            "Sub" => "Operator::Sub",
            // ... etc
            _ => panic!("Unknown operator: {}", op),
        };
        writeln!(code, "    #[angelscript::function(instance, operator = {})]", op_attr);
        writeln!(code, "    pub fn __op_{}(&self, other: &Self) -> Self {{", op.to_lowercase());
        writeln!(code, "        Self({})", expr.replace("self", "self.0").replace("other", "other.0"));
        writeln!(code, "    }}");
    }

    writeln!(code, "}}");

    // Generate module helper
    writeln!(code, "impl {} {{", type_def.name);
    writeln!(code, "    pub fn register_module() -> Module {{");
    writeln!(code, "        Module::new()");
    writeln!(code, "            .ty::<Self>()");
    // ... add all functions
    writeln!(code, "    }}");
    writeln!(code, "}}");

    code
}
```

### Usage

```rust
// In your crate
mod generated;

use generated::vec3::Vec3;

fn setup() {
    let module = Vec3::register_module();
    context.install(module)?;
}
```

### Implementation Complexity

**Pros:**
- Very declarative - just list what you want
- Easy to maintain large APIs
- Generated code is inspectable
- Can add validation in build script
- Could support multiple output formats (Rust, docs, etc.)

**Cons:**
- Requires build.rs complexity
- Configuration parsing/validation
- Need to handle edge cases in code generation
- Method signature inference is hard (may need type hints)
- Slower iteration (rebuild on config change)

### Implementation Files

1. `angelscript-bindgen/src/lib.rs` - Config parsing + code generation library
2. `angelscript-bindgen/src/parser.rs` - TOML/DSL parser
3. `angelscript-bindgen/src/codegen.rs` - Rust code generation
4. Example `build.rs` in documentation

---

## Comparison Matrix

| Aspect | Option 1: Macro | Option 2: Builder | Option 3: Build Script |
|--------|-----------------|-------------------|------------------------|
| **Ergonomics** | Best | Good | Good |
| **Flexibility** | Medium | Best | Good |
| **Compile-time Safety** | Best | Good | Medium |
| **Error Messages** | Poor | Best | Medium |
| **Implementation Effort** | High | Medium | High |
| **New Dependencies** | None | None | toml, possibly syn |
| **Introspection** | No | No | Possible |
| **IDE Support** | Poor | Best | Medium |
| **Maintenance** | Medium | Low | Medium |

## Recommendation

**Start with Option 2 (Builder API)** because:

1. **Lowest implementation effort** - Uses existing macro infrastructure
2. **Best error messages** - Standard Rust trait errors
3. **Most flexible** - Any closure works
4. **Good foundation** - Can build Options 1 and 3 on top of it later
5. **No new proc macros** - Trait impls generated via `macro_rules!`

Then consider:
- **Option 1** later for users who want maximum conciseness
- **Option 3** for large integration projects (e.g., full Bevy bindings crate)

---

## Implementation Plan for Option 2 (Builder API)

### Phase 1: Core Conversion Traits
1. Add `IntoVmSlot` trait to angelscript-core
2. Add `FromVmSlot` trait to angelscript-core
3. Implement for all primitive types

### Phase 2: Native Function Generation Traits
1. Create `IntoNativeConstructor<T, Args>` trait
2. Create `IntoNativeMethod<T, Args, R>` trait
3. Create `IntoNativeOperator<T, Args, R>` trait
4. Use `macro_rules!` to generate impls for arities 0-8

### Phase 3: WrapBuilder Implementation
1. Create `WrapBuilder<T>` struct
2. Implement type kind methods (`.value()`, `.pod()`, `.reference()`)
3. Implement `.constructor()`, `.method()`, `.method_const()`, `.method_mut()`
4. Implement operator methods (`.op_add()`, `.op_sub()`, etc.)
5. Implement `.property()`, `.property_get()`
6. Implement `.static_value()`, `.static_method()`
7. Implement `.build() -> Module`

### Phase 4: Integration
1. Update `Module` to optionally store native function pointers
2. Update `Context::install()` to wire up native functions
3. Add tests with mock external types

### Phase 5: Documentation
1. Add examples in crate docs
2. Create example project wrapping a simple external type
3. Document best practices

### Files to Create/Modify
- `crates/angelscript-core/src/conversion.rs` (new)
- `crates/angelscript-registry/src/wrap_builder.rs` (new)
- `crates/angelscript-registry/src/into_native.rs` (new)
- `crates/angelscript-registry/src/module.rs` (modify)
- `crates/angelscript-registry/src/lib.rs` (modify exports)
- `src/context.rs` (modify install)
