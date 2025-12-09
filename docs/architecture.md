# Architecture Overview

This document provides a high-level overview of the AngelScript-Rust engine architecture.

## Goal

Execute AngelScript code from Rust, with the ability to register Rust types and functions through a macro-based FFI system.

## Target Use Cases

### Pattern A: Engine-Level Scripting
Script IS the game logic. One long-lived module with lots of state.

### Pattern B: Per-Entity Scripting (ECS)
Each entity has a script object instance. Thousands of instances, short execution bursts.

**Key requirement**: Compile once, instantiate many times with isolated state.

## Crate Structure

```
angelscript (main crate)
├── angelscript-core       # Shared types (TypeHash, DataType, TypeEntry, FunctionDef)
├── angelscript-parser     # Lexer, AST, and parser
├── angelscript-macros     # Procedural macros for FFI
├── angelscript-registry   # SymbolRegistry and Module
├── angelscript-compiler   # 2-pass compilation
└── angelscript-modules    # Standard library (string, array, dictionary, math)
```

### Dependency Graph

```
angelscript-core  ←─────────────────────────────┐
       ↑                                        │
       │                                        │
angelscript-parser    angelscript-registry ─────┤
       ↑                     ↑                  │
       │                     │                  │
       └─────── angelscript-compiler ───────────┘
                      ↑
                      │
               angelscript (main)
                      │
            angelscript-modules
```

## Core Concepts

### TypeHash

A deterministic 64-bit hash uniquely identifying types, functions, and methods. Same name always produces the same hash, enabling forward references and unified FFI/script identity. See [Symbol Registry](./symbol-registry.md) for details.

### DataType

Complete type representation including modifiers: `const`, handle (`@`), and reference modes (`&in`, `&out`, `&inout`).

### TypeEntry

Registry storage for all type kinds: primitives, classes, interfaces, enums, funcdefs, and template parameters.

### TypeKind

Memory semantics classification:
- **Value**: Stack-allocated, copied on assignment (optionally POD)
- **Reference**: Heap-allocated via factory, uses ref counting
- **ScriptObject**: Script-defined classes

## FFI Flow

The FFI system connects Rust types to AngelScript through a pipeline:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           COMPILE TIME                                    │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  #[derive(Any)]              #[function]                                 │
│  struct Player { ... }  ───► impl Player { fn update() }                 │
│         │                           │                                    │
│         ▼                           ▼                                    │
│    ClassMeta                  FunctionMeta                               │
│  (type_hash, properties)    (params, return_type, behavior)             │
│         │                           │                                    │
│         └───────────┬───────────────┘                                    │
│                     ▼                                                    │
│              Module::new()                                               │
│                .ty::<Player>()                                           │
│                .function(Player::update__meta)                           │
│                     │                                                    │
└─────────────────────┼────────────────────────────────────────────────────┘
                      │
┌─────────────────────┼────────────────────────────────────────────────────┐
│                     ▼              INITIALIZATION                        │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│              Context::install(module)                                    │
│                     │                                                    │
│                     ▼                                                    │
│    ┌────────────────────────────────────┐                               │
│    │         SymbolRegistry             │                               │
│    ├────────────────────────────────────┤                               │
│    │ types: HashMap<TypeHash, TypeEntry>│ ◄── ClassEntry, EnumEntry... │
│    │ functions: HashMap<TypeHash, Func> │ ◄── FunctionEntry + NativeFn │
│    │ overloads: HashMap<String, Vec<H>> │                               │
│    └────────────────────────────────────┘                               │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
                      │
┌─────────────────────┼────────────────────────────────────────────────────┐
│                     ▼              COMPILATION                           │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│    Script Source ──► Parser ──► AST                                      │
│                                  │                                       │
│                     ┌────────────┴────────────┐                         │
│                     ▼                         ▼                         │
│              Pass 1: Registration      Pass 2: Compilation              │
│              - Register script types   - Type checking                  │
│              - Register signatures     - Overload resolution            │
│              - Build inheritance       - Bytecode generation            │
│                     │                         │                         │
│                     ▼                         ▼                         │
│              Script types added        CompiledModule                   │
│              to SymbolRegistry         (bytecode, metadata)             │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
                      │
┌─────────────────────┼────────────────────────────────────────────────────┐
│                     ▼              RUNTIME                               │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│    Engine (immutable, Arc-shared)                                        │
│      │                                                                   │
│      ├──► SymbolRegistry (FFI + script types/functions)                 │
│      └──► CompiledModules (bytecode)                                    │
│                     │                                                    │
│                     ▼                                                    │
│    Runtime (mutable, per-world)                                          │
│      │                                                                   │
│      ├──► Object Pool (script instances)                                │
│      └──► Global Variables                                              │
│                     │                                                    │
│                     ▼                                                    │
│    VM (transient, reusable)                                              │
│      │                                                                   │
│      ├──► Executes bytecode                                             │
│      ├──► Calls FFI functions via NativeFn                              │
│      └──► Manages call/value stacks                                     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Key Points

1. **Macros generate metadata** - `#[derive(Any)]` produces `ClassMeta`, `#[function]` produces `FunctionMeta`

2. **Module collects metadata** - `Module` is a builder that gathers type and function metadata before installation

3. **Installation populates registry** - `Context::install()` converts metadata to `TypeEntry`/`FunctionEntry` and stores them in `SymbolRegistry`

4. **Compilation uses registry** - Both FFI and script types live in the same registry, enabling seamless interop

5. **Runtime dispatches via NativeFn** - FFI functions store a `NativeFn` pointer that the VM calls directly

## Parser

The parser uses recursive descent to produce a typed AST. Key components:

- **Lexer**: Tokenizes source with span tracking for error messages
- **AST**: Declarations (classes, functions), statements (if, while, for), expressions
- **Parser**: Single-pass recursive descent, produces `Vec<Decl>`

## Compiler

Two-pass compilation:

**Pass 1 - Registration**: Collects all type and function declarations without resolving bodies. Registers script classes, interfaces, enums, funcdefs. Builds inheritance chains and validates template instantiations.

**Pass 2 - Compilation**: Resolves types, performs semantic analysis, generates bytecode. Handles type checking, overload resolution, implicit conversions, and control flow analysis.

## Runtime Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Engine (Arc<Engine>)                       │
│  Immutable after setup, shared across threads                   │
│  Contains: SymbolRegistry, CompiledModules                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Runtime<'engine>                           │
│  Mutable, one per "world"                                       │
│  Contains: Object Pool, Globals, Callbacks                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                            VM                                   │
│  Transient, reusable                                            │
│  Contains: Call Stack, Value Stack                              │
│  Borrows &mut Runtime during execution                          │
└─────────────────────────────────────────────────────────────────┘
```

### Handle

Lightweight 8-byte reference into Runtime's object pool (index + generation for safe reuse).

### Reference Counting

Reference counting is the **VM's responsibility**, not the compiler's. The compiler validates handle types and emits bytecode; the VM tracks counts and manages lifetimes. Native types can provide custom AddRef/Release behaviors.

## Differences from C++ AngelScript

| C++ AngelScript | Our Design |
|-----------------|------------|
| `asIScriptEngine` | `Engine` (Arc-shared) |
| `asIScriptModule` | Compiled into Engine |
| `asIScriptContext` | `Vm` (stateless, reusable) |
| `asIScriptObject` | `Handle` into `Runtime`'s pool |
| Sequential TypeId | Deterministic `TypeHash` |
| Interior mutability | Clear ownership, no `Rc<RefCell<>>` in API |

## Related Documentation

- [Symbol Registry](./symbol-registry.md) - TypeHash, DataType, and SymbolRegistry details
- [FFI Guide](./ffi.md) - Macro reference for registering Rust types
