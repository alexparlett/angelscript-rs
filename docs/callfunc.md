# AngelScript Function Calling System Design

## Overview

This document describes the architecture for calling native/system functions in our Rust AngelScript implementation, modeled after the C++ `as_callfunc.h/cpp` system.

## Status: Phase 1 Complete ✓

The `callfunc` module has been implemented with:
- `SystemFunctionInterface` and `NativeCallable` types
- `SystemFunctionRegistry` for storing all native function implementations
- `call_system_function()` as the unified entry point
- Wrapper helpers for creating callables
- `ScriptObject` behaviour registration via engine methods

## C++ AngelScript Architecture

### Key Components

**`asSSystemFunctionInterface`** - Stored per-function metadata describing how to call it:
```cpp
struct asSSystemFunctionInterface {
    asFUNCTION_t func;           // The actual function pointer
    int baseOffset;              // Offset for virtual methods
    int callConv;                // Calling convention (CDECL, THISCALL, etc.)
    int scriptReturnSize;        // Size of return value
    bool hostReturnInMemory;     // Whether return is via pointer
    bool hostReturnFloat;        // Whether return is float
    // ... more fields for parameter handling
};
```

**`CallSystemFunction()`** - The unified entry point:
```cpp
int CallSystemFunction(int funcId, asCContext *context, void *objectPointer);
```

This function:
1. Looks up `asSSystemFunctionInterface` for the function
2. Prepares arguments from the VM stack
3. Handles the `this` pointer for methods
4. Calls the appropriate platform-specific caller based on calling convention
5. Handles the return value

**`RegisterScriptObject()`** - Registers base script object behaviours:
```cpp
void RegisterScriptObject(asCScriptEngine *engine) {
    engine->scriptTypeBehaviours.engine = engine;
    engine->scriptTypeBehaviours.flags = asOBJ_SCRIPT_OBJECT | asOBJ_REF | asOBJ_GC;
    engine->scriptTypeBehaviours.name = "$obj";
    
    // Uses engine's registration methods, not direct registry manipulation
    engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours, 
        asBEHAVE_ADDREF, "void f()", asMETHOD(asCScriptObject,AddRef), asCALL_THISCALL, 0);
    engine->RegisterBehaviourToObjectType(&engine->scriptTypeBehaviours,
        asBEHAVE_RELEASE, "void f()", asMETHOD(asCScriptObject,Release), asCALL_THISCALL, 0);
    // ... etc
}
```

## Rust Implementation

### File Structure

```
src/
├── core/
│   ├── script_engine.rs      # ScriptEngine with registration methods
│   ├── script_object.rs      # ScriptObject wrapper + register_script_object_behaviours()
│   ├── type_registry.rs      # TypeInfo, FunctionInfo (metadata only, no callables)
│   └── types.rs              # BehaviourType, TypeFlags, etc.
│
├── callfunc/                 # Equivalent to as_callfunc.*
│   ├── mod.rs                # Module exports and documentation
│   ├── interface.rs          # SystemFunctionInterface, NativeCallable, CallConv
│   ├── context.rs            # FunctionCallContext
│   ├── registry.rs           # SystemFunctionRegistry  
│   ├── call.rs               # call_system_function()
│   └── wrappers.rs           # wrap_method, wrap_global helpers
│
└── vm/
    ├── vm.rs                 # Uses call_system_function for CALLSYS
    ├── memory.rs             # ObjectHeap with Object storage
    └── gc.rs                 # GC calls behaviours via call_system_function
```

### Core Types

#### SystemFunctionInterface (interface.rs)

```rust
/// System function interface - describes how to call a native function
/// Equivalent to asSSystemFunctionInterface
pub struct SystemFunctionInterface {
    /// The actual callable (type-erased)
    pub func: NativeCallable,
    
    /// Calling convention
    pub call_conv: CallConv,
    
    /// The Rust TypeId of the 'this' type (for methods)
    /// Used for documentation; actual validation happens at downcast time
    pub this_type: Option<std::any::TypeId>,
    
    /// Parameter type information for marshalling
    pub param_types: Vec<ParamType>,
    
    /// Return type information
    pub return_type: ReturnType,
    
    /// Human-readable name for debugging
    pub name: String,
}

/// Type-erased callable - what actually gets invoked
pub enum NativeCallable {
    /// Function taking FunctionCallContext
    Generic(Arc<dyn Fn(&mut FunctionCallContext) -> Result<(), String> + Send + Sync>),
}

/// Calling convention for native functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallConv {
    CDecl,           // Global function
    ThisCall,        // Method with this pointer
    CDecl_ObjLast,   // Method with this last
    CDecl_ObjFirst,  // Method with this first
    Generic,         // Generic (Rust closures)
}
```

#### FunctionCallContext (context.rs)

```rust
/// Context passed to native function calls
/// Provides access to arguments, this pointer, and return value
pub struct FunctionCallContext<'a> {
    /// The 'this' object for methods (None for global functions)
    this_obj: Option<&'a mut dyn Any>,
    
    /// Arguments from the VM
    args: &'a [ScriptValue],
    
    /// Where to store the return value
    return_value: ScriptValue,
}

impl<'a> FunctionCallContext<'a> {
    /// Get 'this' as mutable concrete type
    pub fn this_mut<T: Any>(&mut self) -> Result<&mut T, String>;
    
    /// Get 'this' as immutable concrete type
    pub fn this_ref<T: Any>(&self) -> Result<&T, String>;
    
    /// Get argument by index
    pub fn arg(&self, index: usize) -> Option<&ScriptValue>;
    
    /// Typed argument accessors
    pub fn arg_i32(&self, index: usize) -> Option<i32>;
    pub fn arg_bool(&self, index: usize) -> Option<bool>;
    pub fn arg_string(&self, index: usize) -> Option<&str>;
    // ... etc
    
    /// Set return value
    pub fn set_return<T: Into<ScriptValue>>(&mut self, value: T);
    
    /// Take return value (consumes it)
    pub fn take_return_value(&mut self) -> ScriptValue;
}
```

#### SystemFunctionRegistry (registry.rs)

```rust
/// Registry of all system/native function implementations
/// This is THE authoritative source for callable native functions.
/// 
/// All native functions (methods, behaviours, property accessors, global functions)
/// are stored here, keyed by FunctionId.
pub struct SystemFunctionRegistry {
    functions: HashMap<FunctionId, SystemFunctionInterface>,
}

impl SystemFunctionRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, func_id: FunctionId, interface: SystemFunctionInterface);
    pub fn get(&self, func_id: FunctionId) -> Option<&SystemFunctionInterface>;
    pub fn contains(&self, func_id: FunctionId) -> bool;
    pub fn len(&self) -> usize;
}
```

### The Call System Function (call.rs)

```rust
/// Call a system function - THE unified entry point
/// Equivalent to CallSystemFunction() in C++
/// 
/// Both VM (via CALLSYS instruction) and GC (via behaviours) use this.
pub fn call_system_function(
    func_id: FunctionId,
    this_obj: Option<&mut dyn Any>,
    args: &[ScriptValue],
    registry: &SystemFunctionRegistry,
) -> Result<ScriptValue, String> {
    // 1. Look up the function interface
    let sys_func = registry.get(func_id)
        .ok_or_else(|| format!("System function {} not found", func_id))?;
    
    // 2. Validate 'this' pointer is provided if required
    // Note: Actual type validation happens in the callable via downcast
    if sys_func.this_type.is_some() && this_obj.is_none() {
        return Err(format!("Method '{}' requires 'this'", sys_func.name));
    }
    
    // 3. Create call context
    let mut ctx = FunctionCallContext::new(this_obj, args);
    
    // 4. Call the function
    match &sys_func.func {
        NativeCallable::Generic(f) => f(&mut ctx)?,
    }
    
    // 5. Return the result
    Ok(ctx.take_return_value())
}
```

**Key Design Decision:** Type checking happens at downcast time inside the callable, not at the `call_system_function` entry point. This avoids lifetime issues with `std::any::TypeId` requiring `'static` bounds on `&dyn Any`.

### Wrapper Helpers (wrappers.rs)

```rust
/// Wrap a method (mutable self, no args)
pub fn wrap_method<T, F, R>(func: F, name: &str) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>;

/// Wrap a method (mutable self, void return)
pub fn wrap_method_void<T, F>(func: F, name: &str) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T) + Send + Sync + 'static;

/// Wrap a method (immutable self)
pub fn wrap_method_const<T, F, R>(func: F, name: &str) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&T) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>;

/// Wrap a method with args slice
pub fn wrap_method_with_args<T, F, R>(func: F, param_count: usize, name: &str) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T, &[ScriptValue]) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>;

/// Wrap a global function (no args)
pub fn wrap_global<F, R>(func: F, name: &str) -> SystemFunctionInterface
where
    F: Fn() -> R + Send + Sync + 'static,
    R: Into<ScriptValue>;

/// Wrap a global function with args
pub fn wrap_global_with_args<F, R>(func: F, param_count: usize, name: &str) -> SystemFunctionInterface
where
    F: Fn(&[ScriptValue]) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>;

/// Wrap a raw function for full control
pub fn wrap_raw<F>(func: F, name: &str) -> SystemFunctionInterface
where
    F: Fn(&mut FunctionCallContext) -> Result<(), String> + Send + Sync + 'static;
```

### ScriptObject Registration (script_object.rs)

Following the C++ pattern, registration uses engine methods:

```rust
/// Register ScriptObject behaviours with the engine
/// Equivalent to RegisterScriptObject() in C++
pub fn register_script_object_behaviours(
    engine: &mut ScriptEngine,
) -> Result<ScriptObjectBehaviourIds, String> {
    // Register the $obj type (like engine->scriptTypeBehaviours in C++)
    let type_id = engine.register_script_object_type(
        "$obj",
        TypeFlags::SCRIPT_OBJECT | TypeFlags::REF_TYPE | TypeFlags::GC_TYPE,
    )?;

    // Use engine's registration methods (not direct registry access)
    let construct_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::Construct,
        "void f(int &in)",
        wrap_method_void::<Object, _>(|_this| {}, "$obj::Construct"),
    )?;

    let add_ref_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::AddRef,
        "void f()",
        wrap_method_void::<Object, _>(|this| { this.add_ref(); }, "$obj::AddRef"),
    )?;

    let release_id = engine.register_object_behaviour_with_impl(
        "$obj",
        BehaviourType::Release,
        "void f()",
        wrap_method::<Object, _, _>(|this| this.release(), "$obj::Release"),
    )?;

    // ... register other behaviours

    Ok(ScriptObjectBehaviourIds { type_id, construct_id, add_ref_id, ... })
}
```

### ScriptEngine Integration (script_engine.rs)

```rust
pub struct ScriptEngine {
    pub registry: Arc<RwLock<TypeRegistry>>,
    modules: HashMap<String, Box<ScriptModule>>,
    
    /// Registry of all native/system function implementations
    system_functions: SystemFunctionRegistry,
    
    /// Cached IDs for ScriptObject behaviours
    script_object_behaviours: Option<ScriptObjectBehaviourIds>,
}

impl ScriptEngine {
    /// Register script object behaviours (call after engine creation)
    pub fn register_script_object(&mut self) -> Result<(), String>;
    
    /// Register method with implementation
    pub fn register_object_method_with_impl(
        &mut self,
        type_name: &str,
        declaration: &str,
        interface: SystemFunctionInterface,
    ) -> Result<FunctionId, String>;
    
    /// Register behaviour with implementation
    pub fn register_object_behaviour_with_impl(
        &mut self,
        type_name: &str,
        behaviour: BehaviourType,
        declaration: &str,
        interface: SystemFunctionInterface,
    ) -> Result<FunctionId, String>;
    
    /// Register global function with implementation
    pub fn register_global_function_with_impl(
        &mut self,
        declaration: &str,
        interface: SystemFunctionInterface,
    ) -> Result<FunctionId, String>;
    
    /// Get the system function registry
    pub fn system_functions(&self) -> &SystemFunctionRegistry;
}
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        ScriptEngine                              │
│  ┌─────────────────────────┐  ┌───────────────────────────────┐ │
│  │     TypeRegistry        │  │   SystemFunctionRegistry      │ │
│  │                         │  │                               │ │
│  │  types (TypeInfo)       │  │  FunctionId → Callable        │ │
│  │  functions (FunctionInfo│  │                               │ │
│  │    - metadata only)     │  │  - Global functions           │ │
│  │  behaviours (FunctionId)│  │  - Methods                    │ │
│  │  properties (FunctionId)│  │  - Behaviours                 │ │
│  │                         │  │  - Property getters/setters   │ │
│  └─────────────────────────┘  └───────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   call_system_function()                         │
│                                                                  │
│  1. Look up SystemFunctionInterface by FunctionId               │
│  2. Validate 'this' is provided if required                     │
│  3. Create FunctionCallContext                                  │
│  4. Invoke NativeCallable (type check happens at downcast)      │
│  5. Return result                                               │
└─────────────────────────────────────────────────────────────────┘
```

## Call Flow Examples

### VM Executing CALLSYS

```
1. VM encounters CALLSYS { func_id }
2. VM gets 'this' from object_register (if method)
3. VM pops args from value_stack
4. VM calls: call_system_function(func_id, this, args, sys_func_registry)
5. call_system_function:
   a. Looks up SystemFunctionInterface
   b. Creates FunctionCallContext
   c. Invokes NativeCallable
   d. Callable does: ctx.this_mut::<Player>()? (type check here)
   e. Returns ScriptValue
6. VM pushes result to value_register
```

### GC Calling Behaviour

```
1. GC needs ref_count for object_id
2. GC gets type_id from GCEntry
3. GC looks up TypeInfo, gets FunctionId for GetRefCount behaviour
4. GC gets object from heap as &mut dyn Any
5. GC calls: call_system_function(func_id, Some(obj), &[], sys_func_registry)
6. call_system_function handles the call uniformly
7. GC extracts i32 from returned ScriptValue
```

## Key Design Decisions

1. **Single entry point**: `call_system_function()` is THE way to call any native function

2. **Type-erased storage**: Functions stored as `NativeCallable` wrapping closures

3. **Downcast at call time**: Type validation happens when callable does `this_mut::<T>()`
   - Avoids `'static` lifetime issues with `std::any::TypeId`
   - Single point of failure with clear error messages

4. **Registration through engine**: Like C++, use `engine.register_*_with_impl()` not direct registry access
   - Ensures metadata (TypeRegistry) and callable (SystemFunctionRegistry) stay in sync
   - Single API for both script and application types

5. **Metadata separate from callables**: 
   - `TypeRegistry` stores type info, function signatures, behaviour IDs
   - `SystemFunctionRegistry` stores actual callable implementations
   - Connected by `FunctionId`

6. **No raw pointers**: Rust's `dyn Any` + downcast instead of C++ `void*`

## Removed from TypeInfo

The following were removed as callables are now in `SystemFunctionRegistry`:
- `rust_methods: HashMap<String, RustMethod>` 
- `rust_accessors: HashMap<String, PropertyAccessor>`

These are replaced by:
- `behaviours: HashMap<BehaviourType, FunctionId>` - stores ID, not callable
- `PropertyInfo.getter/setter: Option<FunctionId>` - stores ID, not callable

## Next Steps

1. ✅ Create `src/callfunc/` module with core types
2. ✅ Implement `call_system_function()`
3. ✅ Implement wrapper helpers
4. ✅ Update `ScriptEngine` with `*_with_impl` registration methods
5. ✅ Implement `register_script_object_behaviours()`
6. ⬜ Update `VM` CALLSYS to use `call_system_function`
7. ⬜ Update `GC` to call behaviours through `call_system_function`
8. ⬜ Update `ObjectHeap` to work with new system
9. ⬜ Remove deprecated registration methods