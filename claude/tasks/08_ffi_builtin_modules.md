# Task 08: Built-in Modules

**Status:** Not Started
**Depends On:** Tasks 01-05, 07
**Estimated Scope:** Implement standard library modules using FFI registration API

---

## Objective

Implement the built-in modules (std, string, array, dictionary, math) using the FFI registration API with declaration string parsing. This replaces the hardcoded implementations in registry.rs.

## Files to Create

- `src/modules/mod.rs` - Module exports and default_modules()
- `src/modules/std.rs` - print, println, eprint, eprintln
- `src/modules/string.rs` - String type with methods
- `src/modules/array.rs` - array<T> template
- `src/modules/dictionary.rs` - dictionary<K,V> template
- `src/modules/math.rs` - Math functions and constants

## Current Hardcoded Implementation

Located in `src/semantic/types/registry.rs` (~3500 lines), including:
- String type with ~20 methods (length, substr, findFirst, etc.)
- String operators (+, ==, !=, <, >, etc.)
- Array template with methods (length, resize, insertLast, etc.)
- Dictionary template

## New Implementation

### mod.rs

```rust
// src/modules/mod.rs

mod std;
mod string;
mod array;
mod dictionary;
mod math;

pub use self::std::std;
pub use self::string::string;
pub use self::array::array;
pub use self::dictionary::dictionary;
pub use self::math::math;

use crate::ffi::{Module, FfiRegistrationError};

/// Returns all default modules
pub fn default_modules() -> Result<Vec<Module<'static>>, FfiRegistrationError> {
    Ok(vec![
        std()?,
        string()?,
        array()?,
        dictionary()?,
        math()?,
    ])
}
```

### std.rs

```rust
// src/modules/std.rs

use crate::ffi::{Module, FfiRegistrationError};

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

### string.rs

```rust
// src/modules/string.rs

use crate::ffi::{Module, FfiRegistrationError};
use crate::runtime::ScriptString;

pub fn string() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();
    module.register_type::<ScriptString>("string")
        .value_type()
        .constructor("void f()", || ScriptString::new())?
        .constructor("void f(const string &in s)", ScriptString::from)?
        // Core methods
        .method("uint length() const", |s: &ScriptString| s.len() as u32)?
        .method("bool isEmpty() const", |s: &ScriptString| s.is_empty())?
        // Substring operations
        .method("string substr(uint start, int count = -1) const", ScriptString::substr)?
        // Search methods
        .method("int findFirst(const string &in str, uint start = 0) const", ScriptString::find_first)?
        .method("int findLast(const string &in str, int start = -1) const", ScriptString::find_last)?
        .method("int findFirstOf(const string &in chars, uint start = 0) const", ScriptString::find_first_of)?
        .method("int findLastOf(const string &in chars, int start = -1) const", ScriptString::find_last_of)?
        // Modification methods
        .method("void insert(uint pos, const string &in str)", ScriptString::insert)?
        .method("void erase(uint pos, int count = -1)", ScriptString::erase)?
        // Operators
        .operator("string opAdd(const string &in)", |a: &ScriptString, b: &str| a.concat(b))?
        .operator("string opAdd_r(const string &in)", |a: &ScriptString, b: &str| {
            let mut result = String::from(b);
            result.push_str(a.as_str());
            result
        })?
        .operator("string& opAddAssign(const string &in)", |a: &mut ScriptString, b: &str| {
            a.push_str(b);
            a
        })?
        .operator("bool opEquals(const string &in)", |a: &ScriptString, b: &str| a.as_str() == b)?
        .operator("int opCmp(const string &in)", |a: &ScriptString, b: &str| a.as_str().cmp(b) as i32)?
        .operator("uint8 opIndex(uint)", |s: &ScriptString, i: u32| s.char_at(i))?
        .build()?;
    Ok(module)
}
```

### array.rs

```rust
// src/modules/array.rs

use crate::ffi::{Module, FfiRegistrationError, TemplateValidation};
use crate::runtime::ScriptArray;

pub fn array() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::root();
    module.register_type::<ScriptArray>("array<class T>")
        .reference_type()
        .template_callback(|_| TemplateValidation::valid())?
        // Factories
        .factory("array<T>@ f()", || ScriptArray::new())?
        .factory("array<T>@ f(uint size)", ScriptArray::with_capacity)?
        .factory("array<T>@ f(uint size, const T &in value)", ScriptArray::filled)?
        .addref(ScriptArray::add_ref)
        .release(ScriptArray::release)
        // Length/size
        .method("uint length() const", ScriptArray::len)?
        .method("bool isEmpty() const", ScriptArray::is_empty)?
        .method("void resize(uint size)", ScriptArray::resize)?
        .method("void reserve(uint size)", ScriptArray::reserve)?
        // Element access
        .method("void insertLast(const T &in value)", array_insert_last)?
        .method("void insertAt(uint index, const T &in value)", array_insert_at)?
        .method("void removeAt(uint index)", ScriptArray::remove_at)?
        .method("void removeLast()", ScriptArray::pop)?
        .method("void removeRange(uint start, uint count)", ScriptArray::remove_range)?
        // Search
        .method("int find(const T &in value) const", array_find)?
        .method("int findByRef(const T &in value) const", array_find_by_ref)?
        // Operators
        .operator("T& opIndex(int index)", array_index)?
        .operator("const T& opIndex(int index) const", array_index_const)?
        .build()?;
    Ok(module)
}

// Generic implementations that handle type-erased values
fn array_insert_last(ctx: &mut CallContext) -> Result<(), NativeError> {
    let array = ctx.this_mut::<ScriptArray>()?;
    let value = ctx.arg_any(0)?;
    array.push(value);
    Ok(())
}

fn array_index(ctx: &mut CallContext) -> Result<(), NativeError> {
    let array = ctx.this_mut::<ScriptArray>()?;
    let index: i32 = ctx.arg(0)?;
    ctx.set_return_ref(array.get_mut(index)?)?;
    Ok(())
}

// ... other helper functions
```

### dictionary.rs

```rust
// src/modules/dictionary.rs

use crate::ffi::{Module, FfiRegistrationError, TemplateValidation};
use crate::runtime::ScriptDict;

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
        // Factory
        .factory("dictionary<K,V>@ f()", || ScriptDict::new())?
        .addref(ScriptDict::add_ref)
        .release(ScriptDict::release)
        // Methods
        .method("void set(const K &in key, const V &in value)", dict_set)?
        .method("bool get(const K &in key, V &out value) const", dict_get)?
        .method("bool exists(const K &in key) const", dict_exists)?
        .method("bool delete(const K &in key)", dict_delete)?
        .method("void clear()", ScriptDict::clear)?
        .method("bool isEmpty() const", ScriptDict::is_empty)?
        .method("uint getSize() const", ScriptDict::len)?
        // Operators
        .operator("V& opIndex(const K &in key)", dict_index)?
        .build()?;
    Ok(module)
}

fn is_hashable(ty: &DataType) -> bool {
    // Primitives, strings, and handles are hashable
    ty.is_primitive() || ty.is_string() || ty.is_handle()
}

// ... helper functions
```

### math.rs

```rust
// src/modules/math.rs

use crate::ffi::{Module, FfiRegistrationError};

pub fn math() -> Result<Module<'static>, FfiRegistrationError> {
    let mut module = Module::new(&["math"]);

    // Constants
    let mut pi = std::f64::consts::PI;
    let mut e = std::f64::consts::E;
    let mut tau = std::f64::consts::TAU;
    module.register_global_property("const double PI", &mut pi)?;
    module.register_global_property("const double E", &mut e)?;
    module.register_global_property("const double TAU", &mut tau)?;

    // Trigonometric functions
    module.register_fn("double sin(double x)", |x: f64| x.sin())?;
    module.register_fn("double cos(double x)", |x: f64| x.cos())?;
    module.register_fn("double tan(double x)", |x: f64| x.tan())?;
    module.register_fn("double asin(double x)", |x: f64| x.asin())?;
    module.register_fn("double acos(double x)", |x: f64| x.acos())?;
    module.register_fn("double atan(double x)", |x: f64| x.atan())?;
    module.register_fn("double atan2(double y, double x)", |y: f64, x: f64| y.atan2(x))?;

    // Hyperbolic functions
    module.register_fn("double sinh(double x)", |x: f64| x.sinh())?;
    module.register_fn("double cosh(double x)", |x: f64| x.cosh())?;
    module.register_fn("double tanh(double x)", |x: f64| x.tanh())?;

    // Exponential and logarithmic
    module.register_fn("double exp(double x)", |x: f64| x.exp())?;
    module.register_fn("double log(double x)", |x: f64| x.ln())?;
    module.register_fn("double log10(double x)", |x: f64| x.log10())?;
    module.register_fn("double log2(double x)", |x: f64| x.log2())?;

    // Power and roots
    module.register_fn("double pow(double base, double exp)", |base: f64, exp: f64| base.powf(exp))?;
    module.register_fn("double sqrt(double x)", |x: f64| x.sqrt())?;
    module.register_fn("double cbrt(double x)", |x: f64| x.cbrt())?;

    // Rounding
    module.register_fn("double floor(double x)", |x: f64| x.floor())?;
    module.register_fn("double ceil(double x)", |x: f64| x.ceil())?;
    module.register_fn("double round(double x)", |x: f64| x.round())?;
    module.register_fn("double trunc(double x)", |x: f64| x.trunc())?;

    // Absolute value and sign
    module.register_fn("double abs(double x)", |x: f64| x.abs())?;
    module.register_fn("int abs(int x)", |x: i32| x.abs())?;
    module.register_fn("double copysign(double x, double y)", |x: f64, y: f64| x.copysign(y))?;

    // Min/max
    module.register_fn("double min(double a, double b)", |a: f64, b: f64| a.min(b))?;
    module.register_fn("double max(double a, double b)", |a: f64, b: f64| a.max(b))?;
    module.register_fn("int min(int a, int b)", |a: i32, b: i32| a.min(b))?;
    module.register_fn("int max(int a, int b)", |a: i32, b: i32| a.max(b))?;

    // Clamp
    module.register_fn("double clamp(double x, double min, double max)",
        |x: f64, min: f64, max: f64| x.clamp(min, max))?;
    module.register_fn("int clamp(int x, int min, int max)",
        |x: i32, min: i32, max: i32| x.clamp(min, max))?;

    Ok(module)
}
```

## Registry Cleanup

Remove from `src/semantic/types/registry.rs`:
- `register_builtin_string()` (~400 lines)
- `register_builtin_template()` for array/dictionary
- All hardcoded method/operator registration

## Usage

```rust
use angelscript::modules;

// Install default modules (all built-ins)
let mut ctx = Context::new();  // Automatically installs default_modules()

// Or install selectively
let mut ctx = Context::new_raw();  // No built-ins
ctx.install(modules::string()?)?;
ctx.install(modules::array()?)?;
ctx.install(modules::math()?)?;  // math::sin(), math::cos(), etc.
```

## Acceptance Criteria

- [ ] All built-in types work through FFI registration
- [ ] Declaration strings parse correctly for all methods
- [ ] Existing tests pass with new implementation
- [ ] Registry.rs reduced by ~800+ lines
- [ ] Context::new() installs all default modules
- [ ] Context::new_raw() creates context without built-ins
- [ ] Individual modules can be installed selectively
- [ ] math namespace works correctly (math::sin, math::PI)
