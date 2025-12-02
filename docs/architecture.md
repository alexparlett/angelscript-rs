# Architecture Design

> **Status**: DRAFT - Pending confirmation

## Goal

Execute AngelScript code from Rust, with the ability to register Rust types and functions.

## Target Use Cases

### Pattern A: Engine-Level Scripting
Script IS the game logic. One long-lived module with lots of state.

### Pattern B: Per-Entity Scripting (ECS)
Each entity has a script object instance. Thousands of instances, short execution bursts.

**Key requirement**: Compile once, instantiate many times with isolated state.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Engine (Arc<Engine>)                       │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Types: Vec3(value), AiAgent(ref), Player(script class)     │ │
│  │ Functions: sqrt, print, spawn_enemy                        │ │
│  │ Compiled: "game.as" → AST/bytecode                         │ │
│  └────────────────────────────────────────────────────────────┘ │
│  Immutable after setup, shared via Arc                          │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Runtime<'engine>                           │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Object Pool:                                               │ │
│  │   [0] AiAgent { native Rust data }                         │ │
│  │   [1] Enemy { script class instance }                      │ │
│  │   [2] AiAgent { native Rust data }                         │ │
│  │   [3] (free)                                               │ │
│  └────────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Globals: game_time=0.0, difficulty=2                       │ │
│  └────────────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ Callbacks: on_damage → FuncRef { ... }                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│  Mutable, one per "world", owns all script objects              │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                            Vm                                   │
│  ┌──────────────────────┐  ┌────────────────────────────────┐  │
│  │ Call Stack:          │  │ Value Stack:                   │  │
│  │   [0] main()         │  │   [0] 42                       │  │
│  │   [1] enemy.update() │  │   [1] Handle(1)                │  │
│  └──────────────────────┘  └────────────────────────────────┘  │
│  Transient, reusable, borrows &mut Runtime during execution     │
└─────────────────────────────────────────────────────────────────┘
```

## Core Types

### Engine

Central registry. Immutable after setup, shared everywhere.

```rust
pub struct Engine {
    /// Type definitions (primitives + registered + script-defined)
    types: Vec<TypeDef>,
    type_by_name: HashMap<String, TypeId>,
    
    /// Global functions (registered + script-defined)
    functions: Vec<FunctionDef>,
    func_by_name: HashMap<String, FunctionId>,
    
    /// Compiled script code
    scripts: HashMap<String, CompiledScript>,
}
```

### Runtime

Owns all script objects for one "world". Mutable.

```rust
pub struct Runtime<'e> {
    engine: &'e Engine,
    objects: ObjectPool,
    globals: HashMap<String, Value>,
    callbacks: HashMap<String, FuncRef>,
}
```

### Vm

Execution machinery. Transient, reusable, stateless between calls.

```rust
pub struct Vm {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
}
```

### Handle

Lightweight reference into Runtime's object pool.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Handle {
    index: u32,
    generation: u32,
}
```

#### Handle Reference Counting

**Important:** Reference counting (AddRef/Release) is the **VM's responsibility**, NOT the compiler's.

- **Compiler Role:** The semantic analyzer validates handle types and emits appropriate bytecode for handle operations (assignment, parameter passing, null checks).
- **VM Role:** The virtual machine tracks reference counts and manages object lifetimes by calling AddRef/Release at runtime.

This separation ensures:
1. The compiler stays type-safe and focused on validation
2. The VM handles runtime memory management details
3. Native types can provide their own AddRef/Release implementations

**Note:** The `@+` auto-handle feature (mentioned in AngelScript docs for FFI) is also a VM-level feature for automatic handle wrapping at native function boundaries, not a compiler-time type modifier.

### Value

Runtime representation of script values.

```rust
#[derive(Clone, Debug)]
pub enum Value {
    // Primitives (inline)
    Void,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    
    // Value types (inline, copied on assignment)
    // e.g., Vec3 stored directly in Value
    
    // Reference types (handle into pool)
    Object(Handle),
    Null,
    
    // Special
    Enum { type_id: TypeId, value: i64 },
    Func(FuncRef),
}
```

### FuncRef

Reference to a callable script function (for funcdefs/callbacks).

```rust
#[derive(Clone)]
pub struct FuncRef {
    function_id: FunctionId,
    bound_object: Option<Handle>,  // For delegates
}
```

## Value Types vs Reference Types

| Kind | Storage | Assignment | Example |
|------|---------|------------|---------|
| Value | Inline in `Value` | Copy | `Vec3`, `int`, `float` |
| Reference | Runtime's object pool | Handle (8 bytes) | `AiAgent`, `Player`, script classes |

```as
// Value type - copied
Vec3 a = Vec3(1, 2, 3);
Vec3 b = a;  // b is a copy

// Reference type - handle
AiAgent@ a = AiAgent();
AiAgent@ b = a;  // b points to same object
```

## Registration API

All registration happens on `Engine` before freezing:

```rust
let mut engine = Engine::new();

// === Global functions ===
engine.register_fn("print", |s: &str| println!("{}", s));
engine.register_fn("sqrt", f64::sqrt);

// Functions that need runtime access
engine.register_fn("spawn_enemy", |runtime: &mut Runtime, pos: Vec3| -> Handle {
    runtime.create("Enemy", &[pos.into()])
});

// === Value type ===
engine.register_type::<Vec3>("Vec3")
    .value_type()
    .constructor(|x: f64, y: f64, z: f64| Vec3::new(x, y, z))
    .property("x", |v: &Vec3| v.x, |v: &mut Vec3, x| v.x = x)
    .property("y", |v: &Vec3| v.y, |v: &mut Vec3, y| v.y = y)
    .property("z", |v: &Vec3| v.z, |v: &mut Vec3, z| v.z = z)
    .method("length", |v: &Vec3| v.length())
    .op_add(|a: &Vec3, b: &Vec3| *a + *b)
    .build()?;

// === Reference type ===
engine.register_type::<AiAgent>("AiAgent")
    .reference_type()
    .constructor(|| AiAgent::new())
    .method("think", |a: &mut AiAgent, dt: f64| a.think(dt))
    .build()?;

// === Enum ===
engine.register_enum("Color")
    .value("Red", 0)
    .value("Green", 1)
    .value("Blue", 2)
    .build()?;

// Compile scripts
engine.compile("game", source)?;

// Freeze
let engine = Arc::new(engine);
```

## Example Usage

### Engine-Level (Pattern A)

```rust
let engine = Arc::new(engine);
let mut runtime = Runtime::new(&engine);
let mut vm = Vm::new();

// Run main game script
vm.call(&mut runtime, "main", &[])?;
```

### Per-Entity ECS (Pattern B)

```rust
let engine = Arc::new(engine);
let mut runtime = Runtime::new(&engine);

// Spawn 1000 goblins - handles into runtime's pool
let goblins: Vec<Handle> = (0..1000)
    .map(|_| runtime.create("GoblinBrain", &[]).unwrap())
    .collect::<Vec<_>>();

// ECS component just holds the handle
struct ScriptComponent {
    handle: Handle,  // 8 bytes
}

// Update loop - reuse Vm
let mut vm = Vm::new();
for goblin in &goblins {
    vm.call_method(&mut runtime, *goblin, "update", &[dt.into()])?;
}
```

## Open Design Questions

These need to be resolved before implementation:

### 1. ScriptObject vs NativeObject
- How do script classes inherit from native types?
- How do interfaces work across the boundary?
- See: [Follow-up #1]

### 2. Memory Management
- When is it safe to drop objects from the pool?
- How do we detect unreachable objects?
- Reference counting? Tracing GC? Manual?
- See: [Follow-up #2]

### 3. Native Function Dispatch
- How to convert Value → Rust types for function calls?
- How to handle various function signatures ergonomically?
- See: [Follow-up #3]

## Implementation Order

1. **Lexer** - Tokenize source (reference C++ tokenizer)
2. **AST** - Define syntax tree nodes
3. **Parser** - Recursive descent parser
4. **Types** - TypeId, TypeDef, basic type system
5. **Engine** - Registration API
6. **Runtime** - Object pool, handles
7. **Vm** - Expression and statement execution
8. **Integration** - Compile and run scripts

## Differences from C++ AngelScript

| C++ AngelScript | Our Design |
|-----------------|------------|
| `asIScriptEngine` | `Engine` (Arc-shared) |
| `asIScriptModule` | Compiled into Engine, no separate type |
| `asIScriptContext` | `Vm` (stateless, reusable) |
| `asIScriptObject` | `Handle` into `Runtime`'s pool |
| Ref counting (AddRef/Release) | TBD - see Follow-up #2 |
| Separate module namespaces | Single namespace (for now) |

## Comparison to Rune.rs

| Rune | Our Design |
|------|------------|
| `Unit` = compiled code | `Engine` holds compiled code |
| `Vm` = execution + heap | Split: `Runtime` (heap) + `Vm` (execution) |
| One Vm per execution | One `Runtime` per world, `Vm` is transient |

The split allows thousands of script objects in one Runtime, with a single reusable Vm for execution.