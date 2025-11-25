# Rune Scripting Language Analysis

## Overview

Rune is an embeddable dynamic programming language for Rust that runs on a stack-based virtual machine. It's designed to feel like "Rust without types" while maintaining practical performance comparable to Lua and Python.

**Key Repository Information:**
- GitHub: https://github.com/rune-rs/rune
- Documentation: https://rune-rs.github.io/
- Crates.io: https://crates.io/crates/rune
- 2.1k stars, 104 forks, 45 contributors

## 1. Native Code Binding

### Function Registration

Rune uses a macro-based approach for registering native functions. Functions are annotated with `#[rune::function]` and registered through modules.

**Basic Function Registration:**

```rust
use rune::Module;

#[rune::function]
fn my_function(a: i64, b: i64) -> i64 {
    a + b
}

pub fn module() -> Result<Module, rune::ContextError> {
    let mut m = Module::default();
    m.function_meta(my_function)?;
    Ok(m)
}
```

**Instance Method Registration:**

```rust
struct MyBytes {
    data: Vec<u8>,
}

#[rune::function(instance)]
fn len(this: &MyBytes) -> usize {
    this.data.len()
}

pub fn module() -> Result<Module, rune::ContextError> {
    let mut m = Module::default();
    m.ty::<MyBytes>()?;  // Register type first
    m.function_meta(len)?;  // Then register methods
    Ok(m)
}
```

### Type Registration

Types must be registered before their methods can be exposed:

```rust
let mut module = Module::default();
module.ty::<MyType>()?;  // Register the type
module.function_meta(MyType::method)?;  // Register methods
```

### Module Installation

Modules are installed into a `Context` before compilation:

```rust
let mut context = Context::with_default_modules()?;
context.install(my_custom_module())?;
```

### Parameter and Return Value Conversion

Rune handles conversion automatically for common Rust types:
- Primitive types: `i64`, `f64`, `bool`, `String`
- Collections: `Vec<T>`, custom structs
- Uses `rune::from_value()` to convert from Rune values to Rust types
- Uses automatic conversion for function parameters through trait implementations

**Real-world Example from JSON Module:**

```rust
#[rune::function]
fn from_string(s: &str) -> Result<Value, Error> {
    let value = serde_json::from_str(s)?;
    Ok(rune::to_value(value)?)
}

#[rune::function]
fn to_string(value: Value) -> Result<String, Error> {
    let value: serde_json::Value = rune::from_value(value)?;
    Ok(serde_json::to_string(&value)?)
}
```

## 2. Global Variables and Constants

### Constants

Constants are registered through modules using `Module::constant()`:

```rust
module.constant("MY_CONSTANT", 42i64)?;
```

### Global State Management

Rune doesn't expose mutable global variables directly. State is managed through:
1. Context - compile-time container for types and functions
2. RuntimeContext - runtime environment (derived from Context)
3. Vm - execution instance (not Send/Sync, single-threaded)

Variables in Rune scripts are:
- Defined with `let` keyword
- All mutable by default (unlike Rust)
- Reference counted for memory safety

## 3. Script Execution Model

### Architecture: Bytecode Compilation + Stack VM

Rune uses a **bytecode compilation** model with a **stack-based virtual machine**.

**Execution Pipeline:**

```
Parse -> Compile -> Bytecode (Unit) -> Execute (VM)
```

### Complete Execution Example

```rust
use rune::{Context, Diagnostics, Source, Sources, Vm};
use rune::termcolor::{ColorChoice, StandardStream};
use std::sync::Arc;

// 1. Create compilation context with modules
let context = Context::with_default_modules()?;
let runtime = Arc::new(context.runtime()?);

// 2. Load source code
let mut sources = Sources::new();
sources.insert(Source::memory("pub fn add(a, b) { a + b }")?);

// 3. Compile with diagnostics
let mut diagnostics = Diagnostics::new();
let result = rune::prepare(&mut sources)
    .with_context(&context)
    .with_diagnostics(&mut diagnostics)
    .build();

// 4. Check for compilation errors
if !diagnostics.is_empty() {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    diagnostics.emit(&mut writer, &sources)?;
}

// 5. Create VM with compiled unit
let unit = result?;
let mut vm = Vm::new(runtime, Arc::new(unit));

// 6. Execute function
let output = vm.call(["add"], (10i64, 20i64))?;
let output: i64 = rune::from_value(output)?;
println!("{}", output); // 30
```

### Simplified API

Rune also provides a simpler API that handles VM creation internally:

```rust
let context = Context::with_default_modules()?;
let mut sources = Sources::new();
sources.insert(Source::memory("pub fn add(a, b) { a + b }")?);

let mut diagnostics = Diagnostics::new();
let mut vm = rune::prepare(&mut sources)
    .with_context(&context)
    .with_diagnostics(&mut diagnostics)
    .build_vm()?;

let output = vm.call(["add"], (10i64, 20i64))?;
let output: i64 = rune::from_value(output)?;
```

### Key Components

- **Context**: Compile-time container for native functions, types, and modules
- **RuntimeContext**: Runtime environment (expensive to create, clone Context data)
- **Sources**: Collection of source files to compile
- **Diagnostics**: Collects compilation errors and warnings
- **Unit**: Compiled bytecode with debug info from compilation
- **Vm**: Virtual machine that executes the Unit

### Execution Methods

The Vm provides multiple execution patterns:

```rust
// Synchronous execution
vm.call(["function"], args)?;
vm.execute(["function"], args)?; // Returns VmExecution for deferred execution
vm.complete()?;  // Run to completion

// Asynchronous execution
vm.async_call(["function"], args).await?;
vm.async_complete().await?;
```

## 4. Error Handling

### Compilation Errors

Rune uses a `Diagnostics` system for rich error reporting:

```rust
let mut diagnostics = Diagnostics::new();
let result = rune::prepare(&mut sources)
    .with_diagnostics(&mut diagnostics)
    .build();

if !diagnostics.is_empty() {
    // Emit formatted diagnostics with source code references
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    diagnostics.emit(&mut writer, &sources)?;
}
```

**Note:** Without attaching diagnostics, build errors don't provide detailed information.

### Runtime Errors

Runtime operations return `Result` types:

```rust
// Function call returns Result
let output = vm.call(["add"], (10i64, 20i64))?;

// Custom functions return Result
#[rune::function]
fn my_function(x: i64) -> Result<i64, MyError> {
    if x < 0 {
        return Err(MyError::NegativeValue);
    }
    Ok(x * 2)
}
```

### Error Type Registration

Custom error types can be registered and used from scripts:

```rust
#[derive(Debug)]
struct Error {
    inner: serde_json::Error,
}

impl Error {
    #[rune::function(protocol = DISPLAY_FMT)]
    fn display(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{}", self.inner);
        VmResult::Ok(())
    }
}

pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::default();
    m.ty::<Error>()?;
    m.function_meta(Error::display)?;
    Ok(m)
}
```

### Panic vs Script Error Distinction

- Rust panics are separate from script errors
- Script errors use `Result<T, E>` with proper error types
- VM operations return `VmResult` for runtime errors
- Diagnostics system provides source location information

## 5. API Design Patterns

### Builder Pattern for Compilation

Rune uses a builder pattern for the compilation pipeline:

```rust
rune::prepare(&mut sources)
    .with_context(&context)
    .with_diagnostics(&mut diagnostics)
    .build()?;
```

### Context Creation Patterns

```rust
// Empty context
let context = Context::new();

// With I/O functions (dbg, print, println)
let context = Context::with_config(true)?;

// With standard library
let context = Context::with_default_modules()?;
```

### Module Pattern

Modules are created through functions that return `Module`:

```rust
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::default();
    // Register types and functions
    m.ty::<MyType>()?;
    m.function_meta(my_function)?;
    Ok(m)
}

// Install into context
context.install(module())?;
```

### Macro-based Registration

The `#[rune::module]` macro simplifies module creation:

```rust
#[rune::module(::my_module)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;
    m.function_meta(from_bytes)?;
    m.function_meta(to_string)?;
    Ok(m)
}
```

### State Isolation

- Context (Send + Sync): Shared compilation-time state
- RuntimeContext (Arc): Shared runtime environment
- Vm (not Send/Sync): Isolated execution instance per thread

## 6. Memory Management

### Reference Counting

Rune uses reference counting for memory safety:

- All non-Copy values are reference counted
- Values can be used at multiple locations
- References are managed transparently

### Ownership Model

```rust
// Context and RuntimeContext use Arc for sharing
let runtime = Arc::new(context.runtime()?);
let unit = Arc::new(compiled_unit);
let vm = Vm::new(runtime, unit);
```

### Avoiding Rc<RefCell<>> in Public API

Rune successfully avoids `Rc<RefCell<>>` in its public API through:

1. **Arc for Sharing**: RuntimeContext and Unit are wrapped in `Arc`
2. **Vm is Single-Threaded**: Vm is not Send/Sync, eliminating need for interior mutability in public API
3. **Value Type Abstraction**: The `Value` type handles internal reference counting without exposing it
4. **Clear Ownership**: Each Vm owns its execution state, no shared mutable state

### Memory Safety Guarantees

From documentation:
> "Rune maintains the same memory safety guarantees as Rust by reference counting values, and unless a value is Copy, values are reference counted and can be used at multiple locations."

### Threading Model

- **Single-threaded VM**: Each Vm instance is not Send/Sync
- **Shared Context**: RuntimeContext is Arc-wrapped and shared across VMs
- **Cross-thread Execution**: `send_execute()` provides Send-compatible execution
- **Multi-VM Support**: Multiple VMs can share the same RuntimeContext

### Stack-based Architecture

Rune uses a stack-based VM with:
- Stack isolation between function calls
- Variables referenced indirectly from a slab
- Efficient constant-time Vm construction

## Key Takeaways for AngelScript-Rust

### Strengths

1. **Clean API**: No `Rc<RefCell<>>` in public interfaces
2. **Macro-based Registration**: `#[rune::function]` captures documentation and metadata
3. **Rich Diagnostics**: Excellent error reporting with source locations
4. **Builder Pattern**: Intuitive compilation pipeline
5. **Type Safety**: Strong integration with Rust type system
6. **Async Support**: First-class async/await support

### Design Principles Applied

1. **Separation of Concerns**:
   - Context (compile-time)
   - RuntimeContext (runtime environment)
   - Unit (compiled code)
   - Vm (execution instance)

2. **Reference Counting over GC**: Simpler, more predictable memory management

3. **Arc for Sharing**: RuntimeContext and Unit are cheaply cloneable through Arc

4. **Module System**: Clean organization of native functionality

5. **Diagnostics Separate from Errors**: Rich error reporting without coupling to Result types

### Potential Improvements

1. **RuntimeContext Creation**: Documentation warns it's "not cheap" - requires cloning from Context
2. **Limited Const Support**: Associated constants not fully supported yet
3. **Single-threaded VM**: Each VM instance cannot be shared across threads (though this simplifies implementation)

## References

- [Rune GitHub Repository](https://github.com/rune-rs/rune)
- [Rune Documentation](https://rune-rs.github.io/)
- [Rune Book](https://rune-rs.github.io/book/)
- [Rune API Documentation](https://docs.rs/rune)
- [Module Documentation](https://docs.rs/rune/latest/rune/struct.Module.html)
- [Context Documentation](https://docs.rs/rune/latest/rune/struct.Context.html)
- [Vm Documentation](https://docs.rs/rune/latest/rune/runtime/struct.Vm.html)
- [Instance Functions Guide](https://rune-rs.github.io/book/instance_functions.html)
- [Rune Modules Source](https://github.com/rune-rs/rune/tree/main/crates/rune-modules)
- [Diagnostics Documentation](https://docs.rs/rune/latest/rune/struct.Diagnostics.html)
