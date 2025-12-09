# Symbol Registry

The Symbol Registry is the central storage system for all types and functions in the AngelScript-Rust engine. It provides O(1) lookup by hash and supports FFI, shared script, and local script entities.

## Overview

```rust
use angelscript_registry::SymbolRegistry;
use angelscript_core::{TypeHash, primitives};

let mut registry = SymbolRegistry::with_primitives();

// O(1) lookup by hash
let int_type = registry.get(primitives::INT32);

// O(1) lookup by name
let player = registry.get_by_name("Player");

// Function lookup with overload support
let overloads = registry.get_function_overloads("print");
```

## Type Identity: TypeHash

`TypeHash` is a deterministic 64-bit hash that uniquely identifies types, functions, methods, and constructors.

### Key Properties

- **Deterministic**: Same name always produces the same hash
- **No order dependencies**: Hash computed before registration
- **Unified identity**: Same hash for FFI and script types with same name
- **O(1) lookups**: Direct hash map access

### Hash Computation

Uses XXHash64 with domain-specific mixing constants to prevent collisions between different entity types.

```rust
use angelscript_core::TypeHash;

// Type hash from qualified name
let player = TypeHash::from_name("Game::Player");
let vec3 = TypeHash::from_name("Vec3");

// Deterministic - same input = same output
assert_eq!(TypeHash::from_name("int"), TypeHash::from_name("int"));
```

### Function Hashes

Function hashes include parameter types to support overloading:

```rust
let int_hash = primitives::INT32;
let float_hash = primitives::FLOAT;
let string_hash = primitives::STRING;

// Different overloads have different hashes
let print_int = TypeHash::from_function("print", &[int_hash]);
let print_str = TypeHash::from_function("print", &[string_hash]);
let print_two = TypeHash::from_function("print", &[int_hash, float_hash]);

assert_ne!(print_int, print_str);
assert_ne!(print_int, print_two);
```

### Method Hashes

Method hashes include the owner type:

```rust
let player_hash = TypeHash::from_name("Player");
let enemy_hash = TypeHash::from_name("Enemy");

// Same method name, different owners = different hashes
let player_update = TypeHash::from_method(
    player_hash,
    "update",
    &[primitives::FLOAT],  // dt parameter
    false,                 // is_const
    false,                 // is_final
);

let enemy_update = TypeHash::from_method(
    enemy_hash,
    "update",
    &[primitives::FLOAT],
    false,
    false,
);

assert_ne!(player_update, enemy_update);
```

### Constructor Hashes

```rust
let vec3_hash = TypeHash::from_name("Vec3");

// Default constructor
let default_ctor = TypeHash::from_constructor(vec3_hash, &[]);

// Parameterized constructor
let xyz_ctor = TypeHash::from_constructor(
    vec3_hash,
    &[primitives::FLOAT, primitives::FLOAT, primitives::FLOAT]
);
```

### Operator Hashes

```rust
let vec3_hash = TypeHash::from_name("Vec3");

// Binary operator: Vec3 + Vec3
let op_add = TypeHash::from_operator(
    vec3_hash,
    "opAdd",
    &[vec3_hash],  // RHS type
    true,          // is_const
    false,         // is_final
);
```

### Template Instance Hashes

```rust
let array_template = TypeHash::from_name("array");
let int_hash = primitives::INT32;
let string_hash = primitives::STRING;

// array<int> and array<string> have different hashes
let array_int = TypeHash::from_template_instance(array_template, &[int_hash]);
let array_str = TypeHash::from_template_instance(array_template, &[string_hash]);

assert_ne!(array_int, array_str);
```

### Domain Constants

Located in `hash_constants` module:

| Constant | Purpose |
|----------|---------|
| `TYPE` | Type domain marker |
| `FUNCTION` | Global function domain |
| `METHOD` | Instance method domain |
| `OPERATOR` | Operator method domain |
| `CONSTRUCTOR` | Constructor domain |
| `IDENT` | Identifier domain |
| `SEP` | Path separator mixing |
| `PARAM_MARKERS[32]` | Per-parameter position constants |

### Primitive Type Hashes

Pre-computed constants in `primitives` module:

```rust
use angelscript_core::primitives;

// Numeric types
primitives::INT8     // int8
primitives::INT16    // int16
primitives::INT32    // int
primitives::INT64    // int64
primitives::UINT8    // uint8
primitives::UINT16   // uint16
primitives::UINT32   // uint
primitives::UINT64   // uint64
primitives::FLOAT    // float
primitives::DOUBLE   // double

// Other primitives
primitives::VOID     // void
primitives::BOOL     // bool
primitives::STRING   // string
primitives::NULL     // null literal type

// Special
primitives::SELF     // Template self-reference
primitives::VARIABLE_PARAM  // Generic `?` type
```

## Data Types: DataType

`DataType` represents a complete type with all modifiers:

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct DataType {
    pub type_hash: TypeHash,       // Base type identity
    pub is_const: bool,            // const qualifier
    pub is_handle: bool,           // @ (reference type)
    pub is_handle_to_const: bool,  // object through handle is const
    pub ref_modifier: RefModifier, // &in, &out, &inout
}

pub enum RefModifier {
    None,   // Value type
    In,     // &in - read-only reference
    Out,    // &out - write-only reference
    InOut,  // &inout - read-write reference
}
```

### Type Modifier Matrix

| AngelScript | Rust Construction | Meaning |
|-------------|-------------------|---------|
| `int` | `DataType::simple(INT32)` | Plain value |
| `const int` | `DataType::with_const(INT32)` | Immutable value |
| `int@` | `DataType::with_handle(INT32, false)` | Handle to mutable object |
| `int@ const` | `DataType::with_handle(INT32, true)` | Handle to const object |
| `const int@` | `DataType::const_handle(INT32, false)` | Const handle to mutable object |
| `const int@ const` | `DataType::const_handle(INT32, true)` | Const handle to const object |
| `int &in` | `DataType::with_ref_in(INT32)` | Read-only reference parameter |
| `int &out` | `DataType::with_ref_out(INT32)` | Write-only reference parameter |
| `int &inout` | `DataType::with_ref_inout(INT32)` | Read-write reference parameter |

### Construction Examples

```rust
use angelscript_core::{DataType, primitives};

// Simple types
let int_type = DataType::simple(primitives::INT32);
let void_type = DataType::void();

// Const types
let const_int = DataType::with_const(primitives::INT32);

// Handle types
let player_hash = TypeHash::from_name("Player");
let handle = DataType::with_handle(player_hash, false);        // Player@
let handle_to_const = DataType::with_handle(player_hash, true); // Player@ const
let const_handle = DataType::const_handle(player_hash, false);  // const Player@

// Reference parameters
let ref_in = DataType::with_ref_in(primitives::FLOAT);
let ref_out = DataType::with_ref_out(primitives::FLOAT);
let ref_inout = DataType::with_ref_inout(primitives::FLOAT);

// Utility methods
assert!(void_type.is_void());
assert!(int_type.is_primitive());
assert!(handle.is_handle);
```

## Type Entries

The registry stores types as `TypeEntry` enum variants:

```rust
pub enum TypeEntry {
    Primitive(PrimitiveEntry),
    Class(ClassEntry),
    Interface(InterfaceEntry),
    Enum(EnumEntry),
    Funcdef(FuncdefEntry),
    TemplateParam(TemplateParamEntry),
}
```

### PrimitiveEntry

Built-in primitive types:

```rust
pub struct PrimitiveEntry {
    pub kind: PrimitiveKind,
    pub type_hash: TypeHash,
}

pub enum PrimitiveKind {
    Void, Bool,
    Int8, Int16, Int32, Int64,
    Uint8, Uint16, Uint32, Uint64,
    Float, Double,
}
```

### ClassEntry

Classes from FFI or script:

```rust
pub struct ClassEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub type_kind: TypeKind,
    pub source: TypeSource,              // FFI or Script

    // Inheritance
    pub base_class: Option<TypeHash>,
    pub interfaces: Vec<TypeHash>,

    // Members
    pub behaviors: TypeBehaviors,         // Constructors, factories, destructor
    pub methods: Vec<TypeHash>,           // Method function hashes
    pub properties: Vec<PropertyEntry>,

    // Template support
    pub template_params: Vec<TypeHash>,   // Non-empty = template definition
    pub template: Option<TypeHash>,       // Set = template instance
    pub type_args: Vec<DataType>,

    // Modifiers
    pub is_final: bool,
    pub is_abstract: bool,
}
```

### Type Kind

Determines memory semantics:

```rust
pub enum TypeKind {
    // Stack-allocated, copied on assignment
    Value {
        size: usize,
        align: usize,
        is_pod: bool,  // Plain Old Data - no constructor/destructor
    },

    // Heap-allocated via factory (FFI types)
    Reference {
        kind: ReferenceKind,
    },

    // Script-defined classes
    ScriptObject,
}

pub enum ReferenceKind {
    Standard,      // Full ref counting with AddRef/Release
    Scoped,        // RAII-style, destroyed at scope exit
    SingleRef,     // Application-controlled lifetime
    GenericHandle, // Type-erased container
}
```

### EnumEntry

Enumeration types:

```rust
pub struct EnumEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub source: TypeSource,
    pub values: Vec<EnumValue>,
}

pub struct EnumValue {
    pub name: String,
    pub value: i64,
}
```

### InterfaceEntry

Interface declarations:

```rust
pub struct InterfaceEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub source: TypeSource,
    pub methods: Vec<TypeHash>,
}
```

### FuncdefEntry

Function pointer types:

```rust
pub struct FuncdefEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub source: TypeSource,
    pub params: Vec<DataType>,
    pub return_type: DataType,
    pub parent_type: Option<TypeHash>,  // For child funcdefs of templates
}
```

## Function Entries

All functions (global, methods, operators, behaviors) stored uniformly:

```rust
pub struct FunctionEntry {
    pub def: FunctionDef,
    pub source: FunctionSource,
    pub implementation: FunctionImpl,
}

pub enum FunctionImpl {
    Native(NativeFn),           // FFI function pointer
    Script { unit: UnitId },    // Script function
    None,                       // Interface method (no implementation)
}
```

### FunctionDef

Complete function signature:

```rust
pub struct FunctionDef {
    pub func_hash: TypeHash,
    pub name: String,
    pub namespace: Vec<String>,
    pub params: Vec<Param>,
    pub return_type: DataType,
    pub owner: Option<TypeHash>,      // For methods
    pub traits: FunctionTraits,
    pub is_variadic: bool,
    pub visibility: Visibility,
}

pub struct Param {
    pub name: String,
    pub data_type: DataType,
    pub default_value: Option<FfiExpr>,
}

pub struct FunctionTraits {
    pub is_const: bool,
    pub is_final: bool,
    pub is_override: bool,
    pub is_private: bool,
    pub is_protected: bool,
    pub is_property: bool,
    pub is_explicit: bool,
}
```

## SymbolRegistry API

### Construction

```rust
// Empty registry
let mut registry = SymbolRegistry::new();

// With all primitives pre-registered
let mut registry = SymbolRegistry::with_primitives();
```

### Type Lookup

```rust
// By hash (O(1))
if let Some(entry) = registry.get(type_hash) {
    match entry {
        TypeEntry::Class(class) => { /* ... */ }
        TypeEntry::Enum(enum_) => { /* ... */ }
        // ...
    }
}

// By qualified name
if let Some(entry) = registry.get_by_name("Game::Player") {
    // ...
}

// Mutable access
if let Some(entry) = registry.get_mut(type_hash) {
    // Modify entry
}

// Check existence
let exists = registry.contains_type(type_hash);
let exists = registry.contains_type_name("Player");
```

### Function Lookup

```rust
// By hash (O(1))
if let Some(func) = registry.get_function(func_hash) {
    let params = &func.def.params;
    let return_type = &func.def.return_type;
}

// All overloads by name
if let Some(overloads) = registry.get_function_overloads("print") {
    for func_hash in overloads {
        if let Some(func) = registry.get_function(*func_hash) {
            // Check if this overload matches
        }
    }
}
```

### Registration

```rust
use angelscript_core::{ClassEntry, EnumEntry, FunctionEntry, TypeKind};

// Register a class
let class = ClassEntry::ffi("Player", TypeKind::reference());
registry.register_type(class.into())?;

// Register an enum
let color = EnumEntry::ffi("Color")
    .with_value("Red", 0)
    .with_value("Green", 1)
    .with_value("Blue", 2);
registry.register_type(color.into())?;

// Register a function
let func_def = FunctionDef::new(
    TypeHash::from_function("print", &[primitives::STRING]),
    "print".to_string(),
    vec![],  // namespace
    vec![Param::new("message", DataType::simple(primitives::STRING))],
    DataType::void(),
    None,    // owner (global function)
    FunctionTraits::default(),
    false,   // is_variadic
    Visibility::Public,
);
registry.register_function(FunctionEntry::ffi(func_def))?;

// Register namespace
registry.register_namespace("Game");
registry.register_namespace("Game::Entities");
```

### Iteration

```rust
// All types
for entry in registry.types() {
    println!("{}", entry.qualified_name());
}

// Specific type kinds
for class in registry.classes() {
    println!("Class: {}", class.name);
}

for enum_ in registry.enums() {
    println!("Enum: {}", enum_.name);
}

for interface in registry.interfaces() {
    println!("Interface: {}", interface.name);
}

// All functions
for func in registry.functions() {
    println!("Function: {}", func.def.name);
}

// Types in namespace
for entry in registry.types_in_namespace("Game") {
    // Only types in Game:: namespace
}
```

### Inheritance Helpers

```rust
// Get inheritance chain (immediate parent to root)
let chain = registry.base_class_chain(warrior_hash);
// Returns: [Player, Entity] for Warrior : Player : Entity

// Get all methods including inherited
let all_methods = registry.all_methods(warrior_hash);

// Get all properties including inherited
let all_props = registry.all_properties(warrior_hash);
```

### Template Support

```rust
use angelscript_core::{TemplateInstanceInfo, TemplateValidation};

// Register validation callback
registry.register_template_callback(
    array_hash,
    Box::new(|info: &TemplateInstanceInfo| {
        if info.sub_types.is_empty() {
            TemplateValidation::invalid("array requires a type argument")
        } else if info.sub_types[0].is_void() {
            TemplateValidation::invalid("array cannot hold void")
        } else {
            TemplateValidation::valid()
        }
    }),
);

// Validate instantiation
let info = TemplateInstanceInfo::new("array", vec![DataType::simple(primitives::INT32)]);
let result = registry.validate_template_instance(&info);
assert!(result.is_valid);
```

## Operator Behaviors

Complete list of operator behaviors for classes:

```rust
pub enum OperatorBehavior {
    // Conversions
    OpConv(TypeHash),        // Explicit: int opConv()
    OpImplConv(TypeHash),    // Implicit: int opImplConv()
    OpCast(TypeHash),        // Explicit handle: Obj@ opCast()
    OpImplCast(TypeHash),    // Implicit handle: Obj@ opImplCast()

    // Unary (prefix)
    OpNeg,      // -a
    OpCom,      // ~a
    OpPreInc,   // ++a
    OpPreDec,   // --a

    // Unary (postfix)
    OpPostInc,  // a++
    OpPostDec,  // a--

    // Binary arithmetic
    OpAdd, OpAddR,    // a + b, b + a (reverse)
    OpSub, OpSubR,
    OpMul, OpMulR,
    OpDiv, OpDivR,
    OpMod, OpModR,
    OpPow, OpPowR,

    // Bitwise
    OpAnd, OpAndR,
    OpOr, OpOrR,
    OpXor, OpXorR,
    OpShl, OpShlR,
    OpShr, OpShrR,
    OpUShr, OpUShrR,

    // Comparison
    OpEquals,    // a == b -> bool
    OpCmp,       // a <=> b -> int (-1, 0, 1)

    // Assignment operators
    OpAssign,       // a = b
    OpAddAssign,    // a += b
    OpSubAssign,    // a -= b
    OpMulAssign,    // a *= b
    OpDivAssign,    // a /= b
    OpModAssign,    // a %= b
    OpPowAssign,    // a **= b
    OpAndAssign,    // a &= b
    OpOrAssign,     // a |= b
    OpXorAssign,    // a ^= b
    OpShlAssign,    // a <<= b
    OpShrAssign,    // a >>= b
    OpUShrAssign,   // a >>>= b

    // Index and call
    OpIndex,        // a[i] (returns reference)
    OpIndexGet,     // get a[i] (returns value)
    OpIndexSet,     // set a[i] = v
    OpCall,         // a(args...)

    // Foreach iteration
    OpForBegin,     // Iterator begin
    OpForEnd,       // Iterator end
    OpForNext,      // Iterator advance
    OpForValue,     // Iterator dereference
    OpForValue0,    // Multi-value iteration
    OpForValue1,
    OpForValue2,
    OpForValue3,
}
```

## Source Tracking

Track whether types/functions come from FFI or script:

```rust
pub enum TypeSource {
    Ffi,                          // Registered from Rust
    Script { unit: UnitId },      // Defined in script
}

pub enum FunctionSource {
    Ffi,
    Script { unit: UnitId },
}
```

## Best Practices

### 1. Use Hash-Based Lookups

```rust
// Good - O(1) lookup
let hash = TypeHash::from_name("Player");
let entry = registry.get(hash);

// Slower - name lookup then hash lookup
let entry = registry.get_by_name("Player");
```

### 2. Cache TypeHash Values

```rust
// Good - compute once
let player_hash = TypeHash::from_name("Player");
let float_hash = primitives::FLOAT;
let update_hash = TypeHash::from_method(player_hash, "update", &[float_hash], false, false);

// Then use cached hashes
registry.get_function(update_hash);

// Avoid - recomputing hashes
registry.get_function(TypeHash::from_method(...)); // Repeated
```

### 3. Use Typed Iteration

```rust
// Good - specific type iteration
for class in registry.classes() {
    // Already know it's a ClassEntry
}

// Less efficient - manual filtering
for entry in registry.types() {
    if let TypeEntry::Class(class) = entry {
        // ...
    }
}
```

### 4. Check Existence Before Registration

```rust
if !registry.contains_type(type_hash) {
    registry.register_type(entry)?;
}
// Or handle the error from register_type
```

## Related Documentation

- [Architecture Overview](./architecture.md) - System architecture
- [FFI Guide](./ffi.md) - Registering Rust types and functions
