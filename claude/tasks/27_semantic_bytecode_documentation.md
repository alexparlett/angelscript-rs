# Task 27: Semantic Analysis & Bytecode Generation Documentation

## Purpose

This document preserves complete knowledge of the `src/semantic/` and `src/codegen/` modules before they are deleted and rebuilt. It captures all the patterns, edge cases, and AngelScript-specific behaviors that have been implemented.

---

## Architecture Overview

### Module Structure

```
src/
├── semantic/
│   ├── mod.rs                      # Main module exports
│   ├── compiler.rs                 # SemanticAnalyzer - orchestrates all passes
│   ├── compilation_context.rs      # CompilationContext - unified FFI+Script lookup
│   ├── template_instantiator.rs    # Template instantiation with caching
│   ├── local_scope.rs              # Local variable tracking with shadowing
│   ├── const_eval.rs               # Compile-time constant evaluation
│   ├── error.rs                    # SemanticError, SemanticErrorKind
│   ├── types/
│   │   ├── mod.rs                  # Re-exports from angelscript-core
│   │   ├── registry.rs             # ScriptRegistry (script-defined types/functions)
│   │   └── conversion.rs           # Type conversion system (DataTypeExt)
│   └── passes/
│       ├── mod.rs                  # Pass exports
│       ├── registration.rs         # Pass 1: Type/function registration
│       └── function_processor/     # Pass 2b: Function compilation
│           ├── mod.rs              # FunctionCompiler main struct
│           ├── expr_checker.rs     # Expression type checking
│           ├── stmt_compiler.rs    # Statement compilation
│           ├── overload_resolver.rs # Function overload resolution
│           ├── type_helpers.rs     # Type resolution helpers
│           └── bytecode_emitter.rs # Conversion instruction emission
│
└── codegen/
    ├── mod.rs                      # Exports
    ├── ir/
    │   └── instruction.rs          # Instruction enum (bytecode IR)
    ├── emitter.rs                  # BytecodeEmitter
    └── module.rs                   # CompiledModule
```

### Compilation Passes

1. **Pass 1 (Registration)**: Walk AST, register all types and functions with empty signatures
2. **Pass 2a (Type Compilation)**: Fill in type details (fields, methods, inheritance)
3. **Pass 2b (Function Compilation)**: Type-check and compile function bodies to bytecode

---

## Type System

### Core Types (in `angelscript-core`)

```rust
pub struct TypeHash(u64);  // Deterministic type identity
pub struct DataType {
    pub type_hash: TypeHash,
    pub is_const: bool,
    pub is_handle: bool,
    pub is_handle_to_const: bool,
    pub ref_modifier: RefModifier,
}

pub enum RefModifier {
    None,
    In,    // &in - read-only reference
    Out,   // &out - write-only reference
    InOut, // &inout - read-write reference
}

pub enum TypeDef {
    Primitive { kind: PrimitiveType, type_hash: TypeHash },
    Class {
        name: String,
        qualified_name: String,
        type_hash: TypeHash,
        fields: Vec<FieldDef>,
        methods: Vec<TypeHash>,
        base_class: Option<TypeHash>,
        interfaces: Vec<TypeHash>,
        operator_methods: FxHashMap<OperatorBehavior, Vec<TypeHash>>,
        properties: FxHashMap<String, PropertyAccessors>,
        is_final: bool,
        is_abstract: bool,
        template_params: Vec<TypeHash>,   // Non-empty = template definition
        template: Option<TypeHash>,        // Set if this is an instance
        type_args: Vec<DataType>,          // Type arguments for instances
        type_kind: TypeKind,
    },
    Interface { name, qualified_name, type_hash, methods: Vec<MethodSignature> },
    Enum { name, qualified_name, type_hash, values: Vec<(String, i64)> },
    Funcdef { name, qualified_name, type_hash, params: Vec<DataType>, return_type: DataType },
    TemplateParam { name, index, owner: TypeHash, type_hash },
}
```

### TypeKind (Memory Semantics)

```rust
pub enum TypeKind {
    // Stack allocated, copied on assignment
    // Uses constructors for instantiation
    Value { size: usize, align: usize, is_pod: bool },

    // Heap allocated via factory, handle semantics
    // Uses factories for instantiation (FFI types like array, dictionary)
    Reference { kind: ReferenceKind },

    // Reference semantics but VM-managed allocation
    // Uses constructors for instantiation (script-defined classes)
    ScriptObject,
}
```

### Type Hash Computation

```rust
// Types are identified by deterministic 64-bit hashes
impl TypeHash {
    pub fn from_name(name: &str) -> Self;
    pub fn from_template_instance(template: TypeHash, args: &[TypeHash]) -> Self;
    pub fn from_method(class: TypeHash, name: &str, params: &[TypeHash], is_const: bool, return_is_const: bool) -> Self;
    pub fn from_constructor(class: TypeHash, params: &[TypeHash]) -> Self;
    pub fn from_function(name: &str, params: &[TypeHash]) -> Self;
}
```

---

## Type Conversion System

### Conversion Kinds

```rust
pub enum ConversionKind {
    Identity,                                    // No conversion needed
    Primitive { from_type, to_type },           // int -> float, etc.
    NullToHandle,                                // null -> T@
    HandleToConst,                               // T@ -> const T@
    DerivedToBase,                               // Derived@ -> Base@
    ClassToInterface,                            // Class@ -> Interface@
    ConstructorConversion { constructor_id },   // User-defined via constructor
    ImplicitConversionMethod { method_id },     // opImplConv()
    ExplicitCastMethod { method_id },           // opCast()
    ImplicitCastMethod { method_id },           // opImplCast()
    ValueToHandle,                               // T -> T@ (for handle initialization)
}
```

### Conversion Costs (for Overload Resolution)

| Cost | Description |
|------|-------------|
| 0 | Exact match / Identity |
| 1 | Primitive implicit widening / null→handle / value→handle |
| 2 | Handle to const / Narrowing (data loss possible) |
| 3 | Derived to base / Float truncation |
| 5 | Class to interface |
| 10 | User-defined implicit conversion |
| 100 | Explicit only (requires cast) |

### Primitive Conversion Rules

All primitive conversions are **implicit** in AngelScript (even narrowing):

```rust
// Integer to Float (implicit, cost 1-2)
int8/16/32 -> float/double: cost 1
int64 -> float: cost 2 (precision loss)
int64 -> double: cost 1

// Float to Integer (implicit, cost 3 - truncation)
float/double -> int8/16/32/64: cost 3

// Integer widening (implicit, cost 1)
int8 -> int16 -> int32 -> int64
uint8 -> uint16 -> uint32 -> uint64

// Integer narrowing (implicit, cost 2)
int64 -> int32 -> int16 -> int8

// Signed/Unsigned reinterpret (same size, cost 2)
int32 <-> uint32
```

### Handle Conversion Rules

```rust
// Adding const is implicit (cost 2)
T@ -> const T@
T@ -> T@ const

// Removing const requires explicit cast
const T@ -> T@  // cost 100, is_implicit=false

// Derived to base (cost 3)
Derived@ -> Base@

// Class to interface (cost 5)
Class@ -> Interface@
```

### Enum Conversions

Enums implicitly convert to/from `int32` (identity conversion, cost 0).

---

## Operator Behaviors

### OperatorBehavior Enum

```rust
pub enum OperatorBehavior {
    // Type conversions
    OpConv(TypeHash),      // T opConv() - explicit value conversion
    OpImplConv(TypeHash),  // T opImplConv() - implicit value conversion
    OpCast(TypeHash),      // T@ opCast() - explicit handle cast
    OpImplCast(TypeHash),  // T@ opImplCast() - implicit handle cast

    // Unary prefix
    OpNeg,      // -obj
    OpCom,      // ~obj (bitwise complement)
    OpPreInc,   // ++obj
    OpPreDec,   // --obj

    // Unary postfix
    OpPostInc,  // obj++
    OpPostDec,  // obj--

    // Binary arithmetic (with reverse variants)
    OpAdd, OpAddR,  // a + b, b.opAdd_r(a)
    OpSub, OpSubR,
    OpMul, OpMulR,
    OpDiv, OpDivR,
    OpMod, OpModR,
    OpPow, OpPowR,

    // Binary bitwise
    OpAnd, OpAndR,  // a & b
    OpOr, OpOrR,    // a | b
    OpXor, OpXorR,  // a ^ b
    OpShl, OpShlR,  // a << b
    OpShr, OpShrR,  // a >> b
    OpUShr, OpUShrR, // a >>> b

    // Comparison
    OpEquals,  // a == b, returns bool
    OpCmp,     // comparison, returns int (-1/0/+1)

    // Assignment
    OpAssign,      // a = b
    OpAddAssign,   // a += b
    // ... etc for all compound assignments

    // Index access
    OpIndex,       // obj[idx] (returns reference)
    OpIndexGet,    // get_opIndex - read-only
    OpIndexSet,    // set_opIndex - write-only

    // Function call
    OpCall,        // obj(args)

    // Foreach iteration
    OpForBegin, OpForEnd, OpForNext,
    OpForValue, OpForValue0-3,
}
```

### Binary Operator Resolution

1. Try left operand's operator (e.g., `left.opAdd(right)`)
2. If not found, try right operand's reverse operator (e.g., `right.opAdd_r(left)`)
3. If both fail, fall back to primitive operation

---

## Type Behaviors (Lifecycle)

```rust
pub struct TypeBehaviors {
    // Constructors (TypeHash is constructor function ID)
    pub constructors: Vec<TypeHash>,
    pub copy_constructor: Option<TypeHash>,

    // Destructor
    pub destructor: Option<TypeHash>,

    // Factories (for reference types)
    pub factories: Vec<TypeHash>,
    pub list_factory: Option<TypeHash>,     // array<T> a = {1, 2, 3}
    pub list_construct: Option<TypeHash>,   // Value type list init

    // Reference counting
    pub add_ref: Option<TypeHash>,
    pub release: Option<TypeHash>,

    // GC support
    pub get_ref_count: Option<TypeHash>,
    pub enum_refs: Option<TypeHash>,
    pub release_refs: Option<TypeHash>,
}
```

---

## Template Instantiation

### Process

1. Check cache for existing instantiation
2. Get template definition (FFI or Script)
3. Validate argument count matches parameter count
4. Run validation callback (FFI templates only)
5. Substitute type parameters with arguments
6. Create specialized operator methods and regular methods
7. Register instance as new Script type
8. Cache the instance

### Type Substitution

```rust
fn substitute_type(
    data_type: &DataType,
    template_params: &[TypeHash],
    args: &[DataType],
    ffi: &FfiRegistry,
    instance_id: Option<TypeHash>,  // For SELF substitution
) -> DataType {
    // Handle primitives::SELF -> instance type
    // Handle T -> concrete arg type
    // Preserve modifiers (const, handle, ref)
}
```

### Template Instance Hash

```rust
// Instance hash = hash(template_hash, arg1_hash, arg2_hash, ...)
let instance_hash = TypeHash::from_template_instance(template_hash, &arg_hashes);
```

---

## Function Compilation

### FunctionCompiler State

```rust
pub struct FunctionCompiler<'ast> {
    context: &'ast CompilationContext<'ast>,  // Type lookup
    local_scope: LocalScope,                   // Variable tracking
    bytecode: BytecodeEmitter,                 // Instruction emission
    return_type: DataType,                     // Expected return type
    namespace_path: Vec<String>,               // Current namespace
    imported_namespaces: Vec<String>,          // Using directives
    current_class: Option<TypeHash>,           // For method compilation
    expected_funcdef_type: Option<TypeHash>,   // Lambda type inference
    expected_init_list_target: Option<TypeHash>, // Init list inference
    errors: Vec<SemanticError>,
}
```

### ExprContext (Expression Result)

```rust
pub struct ExprContext {
    pub data_type: DataType,   // Result type
    pub is_lvalue: bool,       // Can be assigned to
    pub is_mutable: bool,      // Can be modified (if lvalue)
}

impl ExprContext {
    pub fn rvalue(data_type: DataType) -> Self;  // Temporary value
    pub fn lvalue(data_type: DataType, is_mutable: bool) -> Self;
    pub fn const_lvalue(data_type: DataType) -> Self;  // Read-only
}
```

### Local Variable Scoping

```rust
pub struct LocalScope {
    variables: FxHashMap<String, LocalVar>,
    scope_depth: u32,
    shadowed: Vec<(String, LocalVar, u32)>,  // For restoring
    next_offset: u32,
}

// Shadowing: Inner scope variables hide outer scope variables
// When scope exits, outer variables are restored
```

---

## Bytecode IR

### Instruction Set

```rust
pub enum Instruction {
    // Stack operations
    PushInt(i64), PushFloat(f32), PushDouble(f64),
    PushBool(bool), PushNull, PushString(u32),
    Pop, Dup, Swap,

    // Local variables
    LoadLocal(u32), StoreLocal(u32),  // Stack offset

    // Global variables
    LoadGlobal(u32), StoreGlobal(u32),  // Name string index

    // Arithmetic
    Add, Sub, Mul, Div, Mod, Pow,

    // Bitwise
    BitAnd, BitOr, BitXor, BitNot,
    ShiftLeft, ShiftRight, ShiftRightUnsigned,

    // Logical
    LogicalAnd, LogicalOr, LogicalXor, Not,

    // Comparison
    Equal, NotEqual,
    LessThan, LessEqual, GreaterThan, GreaterEqual,

    // Unary
    Negate,
    PreIncrement, PreDecrement,
    PostIncrement, PostDecrement,

    // Control flow
    Jump(i32),           // Relative offset
    JumpIfTrue(i32),
    JumpIfFalse(i32),

    // Function calls
    Call(u64),           // TypeHash of function
    CallMethod(u64),     // TypeHash of method
    CallInterfaceMethod(u64, u32),  // Interface + method index
    CallConstructor { type_id: u64, func_id: u64 },
    CallFactory { type_id: u64, func_id: u64 },
    CallPtr,             // Call through funcdef handle
    Return, ReturnVoid,

    // Object operations
    LoadThis,
    LoadField(u32), StoreField(u32),  // Field index
    StoreHandle,
    ValueToHandle,

    // Type operations
    Cast(TypeHash),
    IsInstanceOf(TypeHash),

    // Primitive type conversions (50+ variants)
    ConvertI8F32, ConvertI16F32, ... // All combinations

    // Handle conversions
    CastHandleToConst,
    CastHandleDerivedToBase,
    CastHandleToInterface,
    CastHandleExplicit,

    // Lambda/Funcdef
    FuncPtr(u64),  // Create handle to function

    // Init list (for array<T> a = {1, 2, 3})
    AllocListBuffer { buffer_var: u32, size: u32 },
    SetListSize { buffer_var: u32, offset: u32, count: u32 },
    PushListElement { buffer_var: u32, offset: u32 },
    SetListType { buffer_var: u32, offset: u32, type_id: u64 },
    FreeListBuffer { buffer_var: u32, pattern_type_id: u64 },

    // Exception handling
    TryStart, TryEnd, CatchStart, CatchEnd,

    // Special
    Nop,
}
```

### BytecodeEmitter

```rust
pub struct BytecodeEmitter {
    instructions: Vec<Instruction>,
    string_constants: Vec<String>,
    next_stack_offset: u32,
    breakable_stack: Vec<BreakableContext>,  // For break/continue
}

impl BytecodeEmitter {
    pub fn emit(&mut self, instruction: Instruction) -> usize;  // Returns position
    pub fn patch_jump(&mut self, position: usize, target: usize);
    pub fn enter_loop(&mut self, continue_target: usize);
    pub fn exit_loop(&mut self, break_target: usize);
    pub fn enter_switch(&mut self);
    pub fn exit_switch(&mut self, break_target: usize);
    pub fn emit_break(&mut self) -> Option<usize>;
    pub fn emit_continue(&mut self) -> Option<usize>;
}
```

---

## Special AngelScript Features

### 1. Lambda Expressions

```angelscript
funcdef int CALLBACK(int);
CALLBACK@ cb = function(x) { return x * 2; };
```

**Implementation:**
- Requires `expected_funcdef_type` context for type inference
- Parameter types inferred from funcdef signature (or explicit)
- Captures all in-scope variables
- Compiled as separate function, returns `FuncPtr` instruction

### 2. Init List Syntax

```angelscript
array<int> arr = {1, 2, 3};
dictionary dict = {{"key", value}};
```

**Implementation:**
- Requires `expected_init_list_target` context
- Type must have `list_factory` (ref types) or `list_construct` (value types) behavior
- Elements compiled and passed to factory/constructor

### 3. Property Accessors

```angelscript
class Foo {
    private int _x;
    int get_x() const { return _x; }
    void set_x(int v) { _x = v; }
}
```

**Implementation:**
- `get_` prefix → getter (const method)
- `set_` prefix → setter (takes value parameter)
- Stored in `TypeDef::Class::properties: FxHashMap<String, PropertyAccessors>`

### 4. Funcdef (Function Handles)

```angelscript
funcdef void HANDLER(int);
HANDLER@ handler = @myFunction;
handler(5);
```

**Implementation:**
- `@function_name` creates `FuncPtr(func_hash)` instruction
- Calling a funcdef variable uses `CallPtr` instruction
- Type matching validates signature compatibility

### 5. Handle Assignment vs Value Assignment

```angelscript
Object@ obj1, obj2;
@obj1 = @obj2;    // Handle assignment (both point to same object)
obj1 = obj2;      // Value assignment (copy)
```

**Implementation:**
- `@target = value` → `StoreHandle` instruction
- `target = value` → Normal assignment with potential copy

### 6. Explicit `super()` Calls

```angelscript
class Derived : Base {
    Derived() { super(args); }
    void foo() { Base::foo(); }  // Call base method
}
```

**Implementation:**
- `super(args)` resolves to base class constructor
- `Base::method()` pattern emits `LoadLocal(0)` (this) + `Call(base_method_hash)`

### 7. Implicit `this` Member Access

```angelscript
class Foo {
    int value;
    void bar() {
        value = 5;  // Implicitly this.value
    }
}
```

**Implementation:**
- Unqualified identifier checked against class fields/properties first
- If found, emits `LoadThis` + `LoadField(idx)` or property getter

### 8. Switch Statement Categories

```rust
enum SwitchCategory {
    Integer,  // int8-64, uint8-64, enum - primitive Equal
    Bool,     // Primitive Equal
    Float,    // Primitive Equal
    String,   // opEquals method call
    Handle,   // Identity comparison + type patterns
}
```

### 9. Foreach Loop

```angelscript
foreach (item : container) { ... }
foreach (key, value : dictionary) { ... }
```

**Implementation:**
- Uses `opForBegin`, `opForEnd`, `opForNext` behaviors
- Single value: `opForValue`
- Multiple values: `opForValue0`, `opForValue1`, etc.

### 10. Interface Method Dispatch

```angelscript
interface IFoo { void bar(); }
IFoo@ obj = @myObj;
obj.bar();  // Dynamic dispatch
```

**Implementation:**
- `CallInterfaceMethod(interface_type_id, method_index)`
- VM looks up actual implementation at runtime

### 11. Type Pattern Matching in Switch (Handles)

```angelscript
switch (handle) {
    case Derived@ d:  // Type pattern
        d.derivedMethod();
        break;
}
```

### 12. Mixin Classes

```angelscript
mixin class MSerializable {
    void serialize() { ... }
}

class Player : MSerializable {
    // Gets serialize() method
}
```

**Implementation:**
- Not real types (stored separately)
- Members copied into classes that include the mixin

---

## Constant Evaluation

```rust
pub enum ConstValue {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
}

impl ConstEvaluator {
    // Supports:
    // - Literals
    // - Binary operations (+, -, *, /, %, **, &, |, ^, <<, >>, ==, !=, <, >, etc.)
    // - Unary operations (-, +, !, ~)
    // - Parentheses
    // - Ternary with constant condition
    // - Enum value references (EnumName::VALUE)

    // Used for:
    // - Enum value expressions
    // - Switch case values
    // - Constant variable initializers
}
```

---

## Overload Resolution

### Algorithm

1. Filter candidates by argument count (considering defaults)
2. Find exact match first (all types match exactly, cost 0)
3. If no exact match, rank by total conversion cost
4. Choose lowest cost match
5. Ambiguous if multiple matches with same cost

### Default Parameters

```rust
// Minimum required = params without defaults
// Maximum = total params
let min_params = func_ref.required_param_count();
let max_params = func_ref.param_count();
arg_count >= min_params && arg_count <= max_params
```

### Reference Parameter Validation

```rust
// &in: accepts any value (lvalue or rvalue)
// &out: requires mutable lvalue
// &inout: requires mutable lvalue
```

---

## Error System

```rust
pub enum SemanticErrorKind {
    TypeMismatch,
    UndefinedType,
    UndefinedVariable,
    UndefinedFunction,
    DuplicateDefinition,
    InvalidOperation,
    WrongArgumentCount,
    WrongTemplateArgCount,
    NotATemplate,
    InvalidTemplateInstantiation,
    VoidExpression,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    MissingListBehavior,
    InternalError,
    // ... etc
}

pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
    pub message: String,
}
```

---

## Key Implementation Patterns

### 1. Two-Registry Architecture

```rust
// FFI types (primitives, native types)
pub struct FfiRegistry { ... }

// Script types (user-defined)
pub struct ScriptRegistry { ... }

// Unified lookup
pub struct CompilationContext {
    ffi: &'a FfiRegistry,
    script: ScriptRegistry,
}

impl CompilationContext {
    fn get_type(&self, hash: TypeHash) -> &TypeDef {
        self.ffi.get_type(hash)
            .or_else(|| self.script.get_type(hash))
            .expect("type not found")
    }
}
```

### 2. TypeHash as Primary Identity

- All type lookups use `TypeHash`
- No `format!()` for type identity in hot paths
- Hashes are computed deterministically from qualified names

### 3. DataType is Copy

```rust
// Good
let hash = data_type.type_hash;
let new_type = DataType::simple(hash);

// Avoid
let hash = data_type.type_hash.clone();  // Unnecessary
```

### 4. Behaviors Stored Separately

```rust
// Behaviors are NOT in TypeDef - stored in registry
registry.get_behaviors(type_hash)
registry.set_behaviors(type_hash, behaviors)
```

### 5. Expression Context Pattern

Always return `ExprContext` from `check_*` methods to track:
- Result type
- Lvalue status (can be target of assignment)
- Mutability (can be modified)

---

## Testing Strategy

### Unit Tests
- Located in `#[cfg(test)]` modules alongside code
- Test individual components (conversion rules, scope tracking, etc.)

### Integration Tests
- `tests/test_harness.rs` - Parser integration tests
- `tests/module_tests.rs` - Full compilation pipeline tests

### Test Scripts
- `test_scripts/*.as` - AngelScript source files covering all features

---

## Future Rebuild Notes

### Critical Files to Reference

1. `crates/angelscript-core/src/type_def.rs` - TypeDef, OperatorBehavior, TypeKind
2. `crates/angelscript-core/src/type_behaviors.rs` - TypeBehaviors struct
3. `crates/angelscript-core/src/data_type.rs` - DataType, RefModifier
4. `crates/angelscript-core/src/type_hash.rs` - Hash computation

### Patterns to Preserve

1. **Two-pass compilation**: Register types first, then compile bodies
2. **Unified context**: Single lookup interface for FFI + Script
3. **TypeHash identity**: Deterministic hashes, no string comparison
4. **Conversion costs**: Proper overload resolution
5. **Operator method dispatch**: Try left operand, then right reverse
6. **Template caching**: Avoid duplicate instantiations
7. **Expected type propagation**: For lambda/init list inference

### Edge Cases to Remember

1. `primitives::NULL` cannot convert to non-handle types
2. `primitives::VARIABLE_PARAM` accepts any type (generic FFI)
3. `primitives::SELF` substituted with instance type in templates
4. Enums are int32 internally
5. Funcdef types are semantically always handles
6. Properties are getter/setter methods with `get_`/`set_` prefix
7. `const auto` preserves const from inferred type
8. Handle initialization (T -> T@) is implicit for classes
