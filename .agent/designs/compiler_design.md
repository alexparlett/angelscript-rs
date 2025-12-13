# AngelScript Compiler - Comprehensive Design Document

## Table of Contents
1. [Overview](#overview)
2. [AngelScript Language Features](#angelscript-language-features)
   - 2.10 [Shared Types and External Declarations](#210-shared-types-and-external-declarations)
   - 2.11 [Inheritance System](#211-inheritance-system)
   - 2.12 [Variable Shadowing](#212-variable-shadowing)
   - 2.13 [Operators and Operator Overloading](#213-operators-and-operator-overloading)
3. [Architecture](#architecture)
4. [Type System](#type-system)
5. [Conversion System](#conversion-system)
6. [Overload Resolution](#overload-resolution)
7. [Bidirectional Type Checking](#bidirectional-type-checking)
8. [Bytecode Design](#bytecode-design)
9. [Data Structures](#data-structures)
10. [Algorithms](#algorithms)
11. [Error Handling](#error-handling)
12. [Testing Strategy](#testing-strategy)
13. [Implementation Tasks](#implementation-tasks)
14. [Performance Considerations](#performance-considerations)
15. [Source Spans and Location Tracking](#source-spans-and-location-tracking)
16. [Debug Tracing Infrastructure](#debug-tracing-infrastructure)

---

## 1. Overview

Build a complete compiler for AngelScript that:
- Parses source → AST (already done in `angelscript-parser`)
- Registers types and functions (Pass 1)
- Type-checks and generates bytecode (Pass 2)
- Integrates with `TypeRegistry` for FFI types

**Design Principles:**
- Fresh design, not copying old compiler patterns
- Each file ≤500 lines
- Each component independently testable
- Use modern Rust idioms
- Performance: O(1) lookups via TypeHash

---

## 2. AngelScript Language Features

### 2.1 Types

| Category | Features |
|----------|----------|
| **Primitives** | `void`, `bool`, `int8/16/32/64`, `uint8/16/32/64`, `float`, `double` |
| **Classes** | Fields, methods, constructors, destructors, inheritance, interfaces |
| **Interfaces** | Method signatures only, multiple inheritance |
| **Enums** | Named integer constants, implicit int32 conversion |
| **Funcdefs** | Function pointer types |
| **Templates** | `array<T>`, `dictionary<K,V>`, validation callbacks |
| **Handles** | `T@` reference-counted pointers, `const T@`, `T@ const` |
| **References** | `&in` (read), `&out` (write), `&inout` (read-write) |

### 2.2 Type Modifiers

```
const int           // Immutable value
int@                // Handle to int (mutable)
const int@          // Handle to const int (can't modify pointee)
int@ const          // Const handle (can't reassign handle)
const int@ const    // Both const
int &in             // Input reference (accepts rvalue or lvalue)
int &out            // Output reference (requires mutable lvalue)
int &inout          // In-out reference (requires mutable lvalue)
```

### 2.3 Expressions

| Category | Expressions |
|----------|-------------|
| **Literals** | Integers, floats, strings, `true`/`false`, `null` |
| **Identifiers** | Variables, enum values, type names |
| **Binary** | `+`, `-`, `*`, `/`, `%`, `**`, `&`, `\|`, `^`, `<<`, `>>`, `>>>` |
| **Comparison** | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| **Logical** | `&&`, `\|\|`, `!`, `^^` (xor) |
| **Assignment** | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `\|=`, `^=`, `<<=`, `>>=`, `>>>=` |
| **Unary** | `-`, `+`, `!`, `~`, `++`, `--` (pre/post) |
| **Member** | `obj.member`, `obj.method()` |
| **Index** | `arr[i]`, `dict["key"]` |
| **Call** | `func(args)`, `obj.method(args)` |
| **Construct** | `Type(args)` |
| **Cast** | `cast<Type>(expr)` |
| **Ternary** | `cond ? then : else` |
| **Lambda** | `function(params) { body }` |
| **Init List** | `{1, 2, 3}`, `{{"a", 1}, {"b", 2}}` |
| **Handle** | `@obj` (get handle), `@obj = @other` (handle assign) |

### 2.4 Statements

| Category | Statements |
|----------|------------|
| **Declarations** | `int x;`, `int x = 5;`, `auto x = expr;` |
| **Control Flow** | `if`/`else`, `switch`/`case`/`default` |
| **Loops** | `while`, `do-while`, `for`, `foreach` |
| **Jump** | `break`, `continue`, `return` |
| **Exception** | `try`/`catch` |
| **Block** | `{ statements }` |

### 2.5 Declarations (Top-Level)

| Declaration | Description |
|-------------|-------------|
| **Function** | `RetType name(params) { body }` |
| **Class** | Fields, methods, constructors, inheritance |
| **Interface** | Method signatures |
| **Enum** | Named constants |
| **Funcdef** | `funcdef RetType NAME(params);` |
| **Namespace** | `namespace Name { ... }` |
| **Mixin** | `mixin class Name { ... }` |
| **Import** | `import void func() from "module";` |
| **Typedef** | `typedef OldType NewType;` |

### 2.6 Operators (Overloadable)

```
// Binary (left.op(right) or right.op_r(left))
opAdd, opSub, opMul, opDiv, opMod, opPow
opAnd, opOr, opXor, opShl, opShr, opUshr
opEquals, opCmp

// Unary
opNeg, opCom, opPreInc, opPreDec, opPostInc, opPostDec

// Assignment
opAssign, opAddAssign, opSubAssign, etc.

// Index
opIndex (returns ref), get_opIndex, set_opIndex

// Call
opCall

// Conversion
opConv, opImplConv (value), opCast, opImplCast (handle)

// Foreach
opForBegin, opForEnd, opForNext, opForValue
```

### 2.7 Behaviors (Lifecycle)

```
// Construction
Constructor      // Type(args)
Factory          // For reference types: Type@ Type(args)
ListConstruct    // Type t = {1, 2, 3}
ListFactory      // Type@ t = {1, 2, 3}
CopyConstructor  // Type(const Type &in)

// Destruction
Destructor       // ~Type()

// Reference counting
AddRef           // Increment ref count
Release          // Decrement ref count

// GC support
GetRefCount, EnumRefs, ReleaseRefs
```

### 2.8 Properties (Virtual)

```angelscript
class Foo {
    private int _x;

    // Virtual property via get_/set_ prefix
    int get_x() const { return _x; }
    void set_x(int v) { _x = v; }
}

// Usage: obj.x (calls get_x or set_x)
```

### 2.9 Special Features

| Feature | Description |
|---------|-------------|
| **Auto type** | `auto x = expr;` infers type from initializer |
| **Const auto** | `const auto x = expr;` preserves const from inferred type |
| **Super** | `super(args)` calls base constructor |
| **Scope resolution** | `Base::method()` calls specific base method |
| **Implicit this** | `field` in method means `this.field` |
| **Default params** | `void foo(int x = 5)` |
| **Named args** | Not supported in AngelScript |

### 2.10 Shared Types and External Declarations

```angelscript
// Shared class - same definition across all modules
shared class SharedData {
    int value;
}

// External declaration - use shared type from another module
external shared class SharedData;

// Shared interface
shared interface ISharedCallback {
    void onEvent();
}

// Shared enum
shared enum SharedState { Ready, Running, Done }
```

**Shared Type Rules:**
- Must have identical definition in all modules that declare it
- Can only reference other shared types
- Cannot have non-shared base class
- Validated at module link time

**Implementation:**
```rust
pub struct ScriptClass {
    // ... existing fields ...
    pub is_shared: bool,
    pub is_external: bool,  // Declared but defined elsewhere
}

// During registration:
// 1. If external: look up existing shared type by name
// 2. If shared: register and mark for cross-module validation
// 3. At link time: verify all shared definitions match
```

### 2.11 Inheritance System

```angelscript
class Base {
    int x;
    void foo() { }
    void bar() { }  // Can be overridden
}

class Derived : Base {
    int y;
    override void bar() { }  // Must use override keyword
    void baz() { }
}

// Multiple interface inheritance
interface IA { void a(); }
interface IB { void b(); }
class Multi : Base, IA, IB {
    void a() { }
    void b() { }
}
```

**Inheritance Resolution Algorithm:**
```
FUNCTION resolve_inheritance(class: ScriptClass, ctx: Context):
    // 1. Resolve base class
    IF class.base_class_name IS Some:
        base_hash = resolve_type_name(class.base_class_name, ctx)?
        base = ctx.get_class(base_hash)?

        // Validate: not final, not interface
        IF base.is_final:
            RETURN Error::CannotInheritFinal

        class.base_class = Some(base_hash)

        // Inherit fields (with offset adjustment)
        class.inherited_field_count = base.total_field_count()

        // Copy virtual method table from base
        class.vtable = base.vtable.clone()

    // 2. Resolve interfaces
    FOR iface_name IN class.interface_names:
        iface_hash = resolve_type_name(iface_name, ctx)?
        iface = ctx.get_interface(iface_hash)?
        class.interfaces.push(iface_hash)

        // Add interface methods to vtable
        FOR method IN iface.methods:
            class.required_methods.push(method)

    // 3. Process own methods
    FOR method IN class.methods:
        IF method.is_override:
            // Find in base or interface
            base_method = find_overridable_method(method.name, method.signature, ctx)?
            class.vtable[base_method.vtable_index] = method.hash
        ELSE:
            // New method - add to vtable
            method.vtable_index = class.vtable.len()
            class.vtable.push(method.hash)

    // 4. Verify all interface methods implemented
    FOR required IN class.required_methods:
        IF NOT is_implemented(required, class):
            IF NOT class.is_abstract:
                RETURN Error::MissingInterfaceMethod
```

**Virtual Method Table:**
```rust
pub struct VTable {
    /// Method hashes indexed by vtable slot
    pub methods: Vec<TypeHash>,

    /// Interface offset table: interface_hash -> start_index
    pub interface_offsets: FxHashMap<TypeHash, usize>,
}

// Call dispatch:
// - Direct call: known method hash -> direct call
// - Virtual call: vtable[slot] -> indirect call
// - Interface call: vtable[interface_offset + slot] -> indirect call
```

### 2.12 Variable Shadowing

```angelscript
void example() {
    int x = 1;        // Slot 0
    {
        int x = 2;    // Slot 1, shadows outer x
        int y = 3;    // Slot 2
        // x here refers to slot 1
    }
    // x here refers to slot 0
    // y is out of scope, slot 2 can be reused
}
```

**Shadowing Implementation:**
```rust
impl LocalScope {
    pub fn declare(&mut self, name: &str, data_type: DataType, is_const: bool) -> u32 {
        let slot = self.next_slot;
        self.next_slot += 1;

        // Save previous binding if exists (for shadowing)
        if let Some(previous) = self.variables.get(name).cloned() {
            self.shadow_stack.push(ShadowedVar {
                name: name.to_string(),
                previous: Some(previous),
                restore_depth: self.depth,
            });
        }

        self.variables.insert(name.to_string(), Local {
            name: name.to_string(),
            data_type,
            slot,
            depth: self.depth,
            is_const,
            is_captured: false,
        });

        slot
    }

    pub fn exit_scope(&mut self) {
        // Restore shadowed variables
        while let Some(shadow) = self.shadow_stack.last() {
            if shadow.restore_depth != self.depth {
                break;
            }
            let shadow = self.shadow_stack.pop().unwrap();
            if let Some(previous) = shadow.previous {
                self.variables.insert(shadow.name, previous);
            } else {
                self.variables.remove(&shadow.name);
            }
        }

        // Reclaim slots from variables at this depth
        self.variables.retain(|_, local| local.depth < self.depth);
        self.depth -= 1;
    }
}
```

### 2.13 Operators and Operator Overloading

**Built-in Operator Precedence (lowest to highest):**
| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 | `=`, `+=`, `-=`, etc. | Right |
| 2 | `?:` (ternary) | Right |
| 3 | `\|\|` | Left |
| 4 | `&&` | Left |
| 5 | `\|` | Left |
| 6 | `^` | Left |
| 7 | `&` | Left |
| 8 | `==`, `!=` | Left |
| 9 | `<`, `<=`, `>`, `>=` | Left |
| 10 | `<<`, `>>`, `>>>` | Left |
| 11 | `+`, `-` | Left |
| 12 | `*`, `/`, `%` | Left |
| 13 | `**` | Right |
| 14 | Unary `-`, `+`, `!`, `~`, `++`, `--` | Right |
| 15 | `.`, `[]`, `()` | Left |

**Operator Method Resolution:**
```
FUNCTION resolve_operator(left: DataType, op: BinaryOp, right: DataType, ctx: Context) -> OperatorResult:
    // 1. Primitive operations (fast path)
    IF is_primitive(left) AND is_primitive(right):
        common = find_common_type(left, right)?
        RETURN OperatorResult::Primitive { result_type: common, op }

    // 2. Try left.opXxx(right)
    method_name = op.to_method_name()  // e.g., "opAdd"
    IF method = find_method(left.type_hash, method_name, [right], ctx):
        RETURN OperatorResult::Method { receiver: left, method }

    // 3. Try right.opXxx_r(left) (reverse)
    reverse_name = op.to_reverse_method_name()  // e.g., "opAdd_r"
    IF method = find_method(right.type_hash, reverse_name, [left], ctx):
        RETURN OperatorResult::ReverseMethod { receiver: right, method }

    // 4. Special: opCmp for comparison operators
    IF op.is_comparison():
        IF method = find_method(left.type_hash, "opCmp", [right], ctx):
            // opCmp returns int: <0, 0, >0
            RETURN OperatorResult::Compare { method, comparison: op }

    // 5. Special: opEquals for == and !=
    IF op == Eq OR op == Ne:
        IF method = find_method(left.type_hash, "opEquals", [right], ctx):
            RETURN OperatorResult::Equals { method, negate: op == Ne }

    RETURN Error::NoOperator { left, op, right }

// Operator method signatures:
// Binary:  T opAdd(const T &in) const
// Reverse: T opAdd_r(const T &in) const
// Compare: int opCmp(const T &in) const  // <0, 0, >0
// Equals:  bool opEquals(const T &in) const
// Index:   T& opIndex(int)  OR  T get_opIndex(int) / void set_opIndex(int, T)
// Call:    T opCall(args...)
```

**Compound Assignment Resolution:**
```
FUNCTION resolve_compound_assign(target: DataType, op: CompoundOp, value: DataType, ctx: Context):
    // 1. Try opXxxAssign (e.g., opAddAssign)
    method_name = op.to_assign_method_name()
    IF method = find_method(target.type_hash, method_name, [value], ctx):
        RETURN CompoundResult::AssignMethod { method }

    // 2. Fall back to: target = target op value
    binary_op = op.to_binary_op()
    binary_result = resolve_operator(target, binary_op, value, ctx)?
    RETURN CompoundResult::Decomposed { binary: binary_result }
```

---

## 3. Architecture

### 3.1 Two-Pass Design

```
┌─────────────────────────────────────────────────────────────┐
│                         Pass 1: Registration                 │
├─────────────────────────────────────────────────────────────┤
│ Input: AST (Script<'ast>)                                   │
│ Output: CompilationContext with all type/function shells    │
│                                                             │
│ 1. Walk all top-level declarations                          │
│ 2. Register types (class, enum, interface, funcdef)         │
│ 3. Register function signatures (no bodies)                 │
│ 4. Resolve inheritance and interface implementation         │
│ 5. Auto-generate constructors/opAssign if needed            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       Pass 2: Compilation                    │
├─────────────────────────────────────────────────────────────┤
│ Input: AST + CompilationContext                             │
│ Output: CompiledModule (bytecode for all functions)         │
│                                                             │
│ For each function body:                                     │
│ 1. Create LocalScope with parameters                        │
│ 2. Type-check all statements (bidirectional)                │
│ 3. Emit bytecode via BytecodeEmitter                        │
│ 4. Collect errors with source locations                     │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Component Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                      CompilationContext                           │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────────┐  │
│  │  TypeRegistry  │  │  ScriptTypes   │  │  ScriptFunctions   │  │
│  │  (FFI types)   │  │  (from Pass 1) │  │  (from Pass 1)     │  │
│  └────────────────┘  └────────────────┘  └────────────────────┘  │
│                                                                   │
│  Queries: resolve_type, get_function, find_method, find_operator │
└──────────────────────────────────────────────────────────────────┘
        │                    │                      │
        ▼                    ▼                      ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────────────┐
│TypeResolver  │    │OverloadRes   │    │  ConversionChecker   │
│TypeExpr→Data │    │Find best fn  │    │  Can A convert to B? │
└──────────────┘    └──────────────┘    └──────────────────────┘
        │                    │                      │
        └────────────────────┼──────────────────────┘
                             ▼
                    ┌──────────────────┐
                    │   ExprChecker    │
                    │  infer / check   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │   StmtCompiler   │
                    │  compile_stmt    │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │ BytecodeEmitter  │
                    │  emit, patch     │
                    └──────────────────┘
```

---

## 4. Type System

### 4.1 DataType (from angelscript-core)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataType {
    pub type_hash: TypeHash,      // Identity of the base type
    pub is_const: bool,           // Value is const
    pub is_handle: bool,          // Is a handle (T@)
    pub is_handle_to_const: bool, // Handle points to const (const T@)
    pub ref_modifier: RefModifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefModifier {
    None,   // Pass by value
    In,     // &in - input reference (accepts rvalue)
    Out,    // &out - output reference (requires lvalue)
    InOut,  // &inout - read-write reference
}
```

### 4.2 TypeHash Computation

```rust
impl TypeHash {
    // Simple type: hash of name
    pub fn from_name(name: &str) -> Self;

    // Template instance: hash(template, args...)
    pub fn from_template_instance(template: TypeHash, args: &[TypeHash]) -> Self;

    // Function: hash(name, param_types...)
    pub fn from_function(name: &str, params: &[TypeHash]) -> Self;

    // Method: hash(class, name, params, is_const)
    pub fn from_method(class: TypeHash, name: &str, params: &[TypeHash], is_const: bool) -> Self;

    // Constructor: hash(class, params)
    pub fn from_constructor(class: TypeHash, params: &[TypeHash]) -> Self;
}
```

### 4.3 ExprInfo (Expression Result)

```rust
#[derive(Debug, Clone, Copy)]
pub struct ExprInfo {
    pub data_type: DataType,
    pub category: ValueCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueCategory {
    /// Temporary value, cannot be assigned to
    Rvalue,
    /// Addressable location, can be assigned to if mutable
    Lvalue { is_mutable: bool },
}

impl ExprInfo {
    pub fn rvalue(data_type: DataType) -> Self;
    pub fn lvalue(data_type: DataType) -> Self;
    pub fn const_lvalue(data_type: DataType) -> Self;

    pub fn is_lvalue(&self) -> bool;
    pub fn is_mutable(&self) -> bool;
    pub fn can_assign(&self) -> bool;
}
```

---

## 5. Conversion System

### 5.1 Conversion Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub kind: ConversionKind,
    pub cost: u32,
    pub is_implicit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// No conversion needed
    Identity,

    /// Primitive type conversion (int -> float, etc.)
    Primitive { from: TypeHash, to: TypeHash },

    /// null -> T@
    NullToHandle,

    /// T@ -> const T@
    HandleToConst,

    /// Derived@ -> Base@
    DerivedToBase { base: TypeHash },

    /// Class@ -> Interface@
    ClassToInterface { interface: TypeHash },

    /// T -> T@ (for handle initialization)
    ValueToHandle,

    /// User-defined via constructor
    Constructor { func: TypeHash },

    /// User-defined via opImplConv
    ImplicitConvMethod { func: TypeHash },

    /// User-defined via opConv (explicit only)
    ExplicitConvMethod { func: TypeHash },

    /// User-defined via opImplCast (handles)
    ImplicitCastMethod { func: TypeHash },

    /// User-defined via opCast (explicit only)
    ExplicitCastMethod { func: TypeHash },
}
```

### 5.2 Conversion Costs

| Cost | Conversion Type | Implicit? |
|------|----------------|-----------|
| 0 | Identity (exact match) | Yes |
| 1 | Primitive widening (int8→int32, float→double) | Yes |
| 1 | Null to handle | Yes |
| 1 | Value to handle (T→T@) | Yes |
| 2 | Primitive narrowing (int32→int8, may lose data) | Yes |
| 2 | Handle to const (T@→const T@) | Yes |
| 2 | Signed/unsigned reinterpret (int32↔uint32) | Yes |
| 3 | Float truncation (float→int) | Yes |
| 3 | Derived to base | Yes |
| 5 | Class to interface | Yes |
| 10 | User-defined implicit (constructor, opImplConv) | Yes |
| 100 | Explicit only (opConv, opCast, const removal) | **No** |

### 5.3 Conversion Algorithm

```
FUNCTION can_convert(source: DataType, target: DataType, ctx: Context) -> Option<Conversion>:
    // 1. Identity check
    IF source == target:
        RETURN Conversion::identity()

    // 2. Same base type, different const (value types)
    IF NOT source.is_handle AND NOT target.is_handle AND source.type_hash == target.type_hash:
        RETURN Conversion::identity()

    // 3. Null to handle
    IF source.type_hash == NULL AND target.is_handle:
        RETURN Conversion::null_to_handle()

    // 4. Null cannot convert to non-handle
    IF source.type_hash == NULL:
        RETURN None

    // 5. VARIABLE_PARAM accepts anything (for FFI generic functions)
    IF target.type_hash == VARIABLE_PARAM:
        RETURN Conversion::identity()

    // 6. Primitive conversions
    IF is_primitive(source) AND is_primitive(target):
        RETURN primitive_conversion(source, target)

    // 7. Enum conversions (enum ↔ int32)
    IF is_enum(source) OR is_enum(target):
        RETURN enum_conversion(source, target, ctx)

    // 8. Funcdef identity (handle flags don't matter)
    IF source.type_hash == target.type_hash AND is_funcdef(source, ctx):
        RETURN Conversion::identity()

    // 9. Value to handle (T → T@)
    IF NOT source.is_handle AND target.is_handle AND source.type_hash == target.type_hash:
        IF is_class_or_interface(source, ctx):
            RETURN Conversion::value_to_handle()

    // 10. Handle conversions
    IF source.is_handle AND target.is_handle:
        RETURN handle_conversion(source, target, ctx)

    // 11. User-defined conversions
    RETURN user_defined_conversion(source, target, ctx)
```

### 5.4 Primitive Conversion Matrix

```
FROM \ TO   | int8 | int16 | int32 | int64 | uint8 | uint16 | uint32 | uint64 | float | double
------------|------|-------|-------|-------|-------|--------|--------|--------|-------|--------
int8        |  0   |   1   |   1   |   1   |   2   |    2   |    2   |    2   |   1   |    1
int16       |  2   |   0   |   1   |   1   |   2   |    2   |    2   |    2   |   1   |    1
int32       |  2   |   2   |   0   |   1   |   2   |    2   |    2   |    2   |   1   |    1
int64       |  2   |   2   |   2   |   0   |   2   |    2   |    2   |    2   |   2   |    1
uint8       |  2   |   2   |   2   |   2   |   0   |    1   |    1   |    1   |   1   |    1
uint16      |  2   |   2   |   2   |   2   |   2   |    0   |    1   |    1   |   1   |    1
uint32      |  2   |   2   |   2   |   2   |   2   |    2   |    0   |    1   |   1   |    1
uint64      |  2   |   2   |   2   |   2   |   2   |    2   |    2   |    0   |   2   |    1
float       |  3   |   3   |   3   |   3   |   3   |    3   |    3   |    3   |   0   |    1
double      |  3   |   3   |   3   |   3   |   3   |    3   |    3   |    3   |   2   |    0

All primitive conversions are IMPLICIT (is_implicit = true)
```

---

## 6. Overload Resolution

### 6.1 Algorithm

```
FUNCTION resolve_overload(candidates: [FunctionDef], args: [DataType], ctx: Context) -> Result<TypeHash>:
    // Step 1: Filter to viable candidates
    viable = []
    FOR func IN candidates:
        min_params = func.required_param_count()  // Params without defaults
        max_params = func.param_count()
        IF args.len() >= min_params AND args.len() <= max_params:
            viable.push(func)

    IF viable.is_empty():
        RETURN Error::NoViableCandidate

    // Step 2: Check for exact match (fast path)
    FOR func IN viable:
        IF is_exact_match(func, args):
            RETURN Ok(func.hash)

    // Step 3: Calculate conversion costs
    ranked = []
    FOR func IN viable:
        cost = total_conversion_cost(func, args, ctx)
        IF cost IS Some:
            ranked.push((func.hash, cost))

    IF ranked.is_empty():
        RETURN Error::NoViableCandidate

    // Step 4: Sort by cost (ascending)
    ranked.sort_by_key(|(_, cost)| cost)

    // Step 5: Check for ambiguity
    IF ranked.len() >= 2 AND ranked[0].cost == ranked[1].cost:
        RETURN Error::Ambiguous(ranked[0].hash, ranked[1].hash)

    RETURN Ok(ranked[0].hash)


FUNCTION total_conversion_cost(func: FunctionDef, args: [DataType], ctx: Context) -> Option<u32>:
    total = 0
    FOR (param, arg) IN zip(func.params, args):
        conv = can_convert(arg, param.data_type, ctx)
        IF conv IS None OR NOT conv.is_implicit:
            RETURN None  // Cannot use this overload
        total += conv.cost
    RETURN Some(total)
```

### 6.2 Operator Resolution

```
FUNCTION resolve_binary_operator(left: DataType, right: DataType, op: BinaryOp, ctx: Context) -> Option<OperatorResult>:
    // 1. Try left.op(right)
    IF method = find_operator(left.type_hash, op.to_behavior(), ctx):
        func = ctx.get_function(method)
        IF can_convert(right, func.params[0].data_type, ctx).is_some_and(|c| c.is_implicit):
            RETURN Some(OperatorResult::Method { receiver: left, method })

    // 2. Try right.op_r(left) (reverse)
    IF method = find_operator(right.type_hash, op.to_reverse_behavior(), ctx):
        func = ctx.get_function(method)
        IF can_convert(left, func.params[0].data_type, ctx).is_some_and(|c| c.is_implicit):
            RETURN Some(OperatorResult::ReverseMethod { receiver: right, method })

    // 3. Primitive fallback
    IF is_primitive_op(left, right, op):
        RETURN Some(OperatorResult::Primitive)

    RETURN None
```

---

## 7. Bidirectional Type Checking

### 7.1 Core Pattern

```rust
impl ExprChecker {
    /// Synthesize type from expression (infer mode)
    /// Used when we don't know what type to expect
    pub fn infer(&mut self, expr: &Expr) -> Result<ExprInfo, ()>;

    /// Check expression against expected type (check mode)
    /// Used when we know what type we need
    pub fn check(&mut self, expr: &Expr, expected: &DataType) -> Result<ExprInfo, ()>;
}
```

### 7.2 When to Use Each Mode

| Context | Mode | Why |
|---------|------|-----|
| Variable init with type | check | `int x = expr` - we know we need int |
| Variable init with auto | infer | `auto x = expr` - infer from expr |
| Return statement | check | We know the function's return type |
| Function argument | check | We know the parameter type |
| Binary operand | infer | Need to find operator first |
| Condition | check | We know we need bool |
| Lambda body | check | We know expected funcdef signature |
| Init list | check | We know expected container type |

### 7.3 Check Mode Implementation

```
FUNCTION check(expr: Expr, expected: DataType) -> Result<ExprInfo>:
    MATCH expr:
        // Lambda can use expected type for parameter inference
        Lambda(lambda):
            RETURN check_lambda(lambda, expected)

        // Init list uses expected type to determine container
        InitList(list):
            RETURN check_init_list(list, expected)

        // For most expressions: infer then coerce
        _:
            info = infer(expr)?
            coerce(info, expected)?
            RETURN info


FUNCTION coerce(info: ExprInfo, target: DataType) -> Result<()>:
    IF info.data_type == target:
        RETURN Ok(())

    conv = can_convert(info.data_type, target, ctx)
    IF conv IS None OR NOT conv.is_implicit:
        RETURN Error::TypeMismatch { got: info.data_type, expected: target }

    emit_conversion(conv)
    RETURN Ok(())
```

### 7.4 Lambda Type Inference

```
FUNCTION check_lambda(lambda: LambdaExpr, expected: DataType) -> Result<ExprInfo>:
    // Get funcdef from expected type
    funcdef = ctx.get_type(expected.type_hash).as_funcdef()
        .ok_or(Error::ExpectedFuncdef)?

    // Infer parameter types from funcdef signature
    param_types = []
    FOR (i, param) IN lambda.params.enumerate():
        IF param.type_expr IS Some:
            // Explicit type - resolve it
            param_types.push(resolve_type(param.type_expr))
        ELSE:
            // Infer from funcdef
            param_types.push(funcdef.params[i])

    // Create unique function hash for this lambda
    lambda_hash = TypeHash::from_lambda(current_function, next_lambda_id++)

    // Compile lambda body with inferred types
    compile_lambda(lambda_hash, param_types, funcdef.return_type, lambda.body)

    // Emit FuncPtr instruction
    emitter.emit_func_ptr(lambda_hash)

    RETURN ExprInfo::rvalue(expected)
```

---

## 8. Bytecode Design

### 8.1 Storage Architecture

**Bytecode** is stored in the registry (`FunctionImpl::Script.bytecode`).
**Constants** are stored at module level (`CompiledModule.constants`) to avoid duplication.

```rust
/// Per-function bytecode (stored in FunctionImpl::Script)
pub struct BytecodeChunk {
    /// Raw bytecode bytes (references constants by index)
    pub code: Vec<u8>,
    /// Line numbers (parallel to code, for error reporting)
    pub lines: Vec<u32>,
}

/// Bytecode stored in registry function entries
pub enum FunctionImpl {
    Native(Option<NativeFn>),
    Script {
        unit_id: UnitId,
        bytecode: Option<BytecodeChunk>,  // Set after compilation
    },
    Abstract,
    External { module: String },
}

/// Module-level constant pool with deduplication
pub struct ConstantPool {
    constants: Vec<Constant>,
    index: FxHashMap<ConstantKey, u32>,  // For deduplication
}

pub enum Constant {
    Int(i64),
    Uint(u64),
    Float32(f32),
    Float64(f64),
    String(String),
    TypeHash(TypeHash),
}

/// Compilation result (constants + tracking)
pub struct CompiledModule {
    pub constants: ConstantPool,    // Shared, needed at runtime
    pub functions: Vec<TypeHash>,   // What was compiled (bytecode in registry)
    pub global_inits: Vec<TypeHash>,
}
```

**Data flow:**
1. Compiler emits bytecode referencing constants by index
2. Bytecode stored in `registry.get_function_mut(hash).implementation.set_bytecode(chunk)`
3. Constants stored in `CompiledModule.constants`
4. VM lookup: `registry.get_function(hash).implementation.bytecode()` + `module.constants()`

**Benefits:**
- Strings appearing in multiple functions are stored once
- Single registry lookup for function + bytecode
- Type hashes deduplicated automatically

### 8.2 Instruction Format

```
Single byte opcodes:
  [opcode]

One-byte operand:
  [opcode] [u8]

Two-byte operand (for jumps):
  [opcode] [u8 low] [u8 high]

Three-byte operand (for long constants):
  [opcode] [u8] [u8] [u8]
```

### 8.3 OpCode Enum

```rust
#[repr(u8)]
pub enum OpCode {
    // === Constants ===
    Constant,       // [idx:u8] Push constant from pool
    ConstantLong,   // [idx:u24] For constant pools > 256
    PushNull,       // Push null
    PushTrue,       // Push true
    PushFalse,      // Push false

    // === Stack ===
    Pop,            // Discard top
    PopN,           // [n:u8] Discard n values
    Dup,            // Duplicate top

    // === Locals ===
    GetLocal,       // [slot:u8] Load local
    SetLocal,       // [slot:u8] Store to local
    GetLocalLong,   // [slot:u16] For > 256 locals
    SetLocalLong,   // [slot:u16]

    // === Globals ===
    GetGlobal,      // [idx:u8] Load global by constant index
    SetGlobal,      // [idx:u8] Store to global

    // === Fields ===
    GetField,       // [idx:u8] Load field from object on stack
    SetField,       // [idx:u8] Store to field
    GetThis,        // Push this reference

    // === Arithmetic (type-specific) ===
    AddI32, AddI64, AddF32, AddF64,
    SubI32, SubI64, SubF32, SubF64,
    MulI32, MulI64, MulF32, MulF64,
    DivI32, DivI64, DivF32, DivF64,
    ModI32, ModI64,
    PowF32, PowF64,
    NegI32, NegI64, NegF32, NegF64,

    // === Bitwise ===
    BitAnd, BitOr, BitXor, BitNot,
    Shl, Shr, Ushr,

    // === Comparison ===
    EqI32, EqI64, EqF32, EqF64, EqBool,
    NeI32, NeI64, NeF32, NeF64, NeBool,
    LtI32, LtI64, LtF32, LtF64,
    LeI32, LeI64, LeF32, LeF64,
    GtI32, GtI64, GtF32, GtF64,
    GeI32, GeI64, GeF32, GeF64,

    // === Logical ===
    Not,            // Logical not (bool)

    // === Jumps ===
    Jump,           // [offset:i16] Unconditional
    JumpIfTrue,     // [offset:i16] Pop, jump if true
    JumpIfFalse,    // [offset:i16] Pop, jump if false
    Loop,           // [offset:u16] Jump backward

    // === Calls ===
    Call,           // [idx:u8] Call function (hash in constants)
    CallMethod,     // [idx:u8] Call method
    CallVirtual,    // [idx:u8] [method:u8] Interface dispatch
    CallPtr,        // Call through funcdef handle on stack
    Return,         // Return with value
    ReturnVoid,     // Return void

    // === Objects ===
    New,            // [type:u8] [ctor:u8] Allocate + construct
    NewFactory,     // [type:u8] [factory:u8] Call factory

    // === Conversions ===
    // Integer conversions (24 opcodes)
    I8toI16, I8toI32, I8toI64, I16toI32, I16toI64, I32toI64,
    I64toI32, I64toI16, I64toI8, I32toI16, I32toI8, I16toI8,
    U8toU16, U8toU32, U8toU64, U16toU32, U16toU64, U32toU64,
    U64toU32, U64toU16, U64toU8, U32toU16, U32toU8, U16toU8,

    // Float conversions (20 opcodes)
    I32toF32, I32toF64, I64toF32, I64toF64,
    U32toF32, U32toF64, U64toF32, U64toF64,
    F32toI32, F32toI64, F64toI32, F64toI64,
    F32toU32, F32toU64, F64toU32, F64toU64,
    F32toF64, F64toF32,
    I8toF32, I16toF32,  // etc...

    // Signed/unsigned reinterpret
    I32toU32, U32toI32, I64toU64, U64toI64,

    // Handle conversions
    HandleToConst,
    DerivedToBase,    // [base_type:u8]
    ClassToInterface, // [interface:u8]
    ValueToHandle,

    // === Type checks ===
    InstanceOf,       // [type:u8] Check if handle is instance
    Cast,             // [type:u8] Explicit cast (may throw)

    // === Funcdef ===
    FuncPtr,          // [func:u8] Create function pointer

    // === Increment/Decrement ===
    PreIncI32, PreIncI64, PreDecI32, PreDecI64,
    PostIncI32, PostIncI64, PostDecI32, PostDecI64,
}
```

### 8.4 BytecodeEmitter

```rust
/// Emits bytecode for a single function.
/// Uses a shared module-level constant pool (passed by reference).
pub struct BytecodeEmitter<'pool> {
    chunk: BytecodeChunk,
    constants: &'pool mut ConstantPool,  // Shared module-level pool
    current_line: u32,

    /// Stack of loop/switch contexts for break/continue
    breakable_stack: Vec<BreakableContext>,
}

struct BreakableContext {
    kind: BreakableKind,
    continue_target: Option<usize>,
    break_patches: Vec<usize>,
}

impl<'pool> BytecodeEmitter<'pool> {
    pub fn new(constants: &'pool mut ConstantPool) -> Self;

    // Emit instructions
    pub fn emit(&mut self, op: OpCode);
    pub fn emit_byte(&mut self, byte: u8);
    pub fn emit_u16(&mut self, value: u16);
    pub fn emit_constant(&mut self, constant: Constant) -> u32;  // Index into shared pool

    // Jumps
    pub fn emit_jump(&mut self, op: OpCode) -> usize;  // Returns patch location
    pub fn emit_loop(&mut self, target: usize);
    pub fn patch_jump(&mut self, location: usize);

    // Loops
    pub fn begin_loop(&mut self);
    pub fn set_continue_target(&mut self, target: usize);
    pub fn end_loop(&mut self);
    pub fn emit_break(&mut self) -> Result<()>;
    pub fn emit_continue(&mut self) -> Result<()>;

    // Finalize
    pub fn finish(self) -> BytecodeChunk;
}
```

---

## 9. Data Structures

### 9.1 Using Existing TypeRegistry (IMPORTANT)

**The compiler uses the existing unified type system from `angelscript-core` and `angelscript-registry`.**
No new type structures - script types use the same `ClassEntry`, `FunctionEntry`, etc. as FFI types.

```rust
// Existing types from angelscript-core:
pub enum TypeEntry {
    Primitive(PrimitiveEntry),
    Class(ClassEntry),      // Used for script classes too
    Enum(EnumEntry),        // Used for script enums too
    Interface(InterfaceEntry),
    Funcdef(FuncdefEntry),
    TemplateParam(TemplateParamEntry),
}

pub struct ClassEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub type_kind: TypeKind,
    pub source: TypeSource,       // FFI vs Script
    pub base_class: Option<TypeHash>,
    pub interfaces: Vec<TypeHash>,
    pub behaviors: TypeBehaviors,
    pub methods: Vec<TypeHash>,   // Function hashes (actual FunctionEntry in registry)
    pub properties: Vec<PropertyEntry>,  // Virtual properties AND direct fields
    pub template_params: Vec<TypeHash>,  // Hashes to TemplateParamEntry (e.g., "array::T")
    pub template: Option<TypeHash>,
    pub type_args: Vec<DataType>,
    pub is_final: bool,
    pub is_abstract: bool,
}

pub struct FunctionEntry {
    pub def: FunctionDef,
    pub implementation: FunctionImpl,
    pub source: FunctionSource,
}

pub enum FunctionImpl {
    Native(Option<NativeFn>),
    Script {
        unit_id: UnitId,
        bytecode: Option<BytecodeChunk>,  // Set after compilation
    },
    Abstract,
    External { module: String },
}
```

**Script vs FFI distinction via TypeSource and FunctionSource:**
```rust
// For types (classes, enums, interfaces, etc.):
pub enum TypeSource {
    Ffi { type_id: Option<TypeId> },
    Script { unit_id: UnitId, span: Span },
}

// For functions (separate enum - unit_id is in FunctionImpl::Script):
pub enum FunctionSource {
    Ffi,
    Script { span: Span },
}

// Creating script types:
let class = ClassEntry::script("Player", "Game::Player", TypeSource::script(unit_id, span));
let function = FunctionEntry::script(func_def, unit_id, FunctionSource::script(span));
```

### 9.2 CompilationContext

The compiler wraps `TypeRegistry` with additional state needed during compilation:

```rust
pub struct CompilationContext<'reg> {
    /// The unified registry (FFI + script types)
    /// Script types are registered here during Pass 1
    registry: &'reg mut TypeRegistry,

    /// Current compilation unit ID
    unit_id: UnitId,

    /// AST reference for body compilation
    ast: &'reg Script<'reg>,

    /// Function body locations in AST (for Pass 2)
    function_bodies: FxHashMap<TypeHash, AstIndex>,

    /// Template instance cache (for lazy instantiation)
    template_instances: FxHashMap<TemplateKey, TypeHash>,

    /// Current namespace during compilation
    current_namespace: Vec<String>,

    /// Error collector
    errors: ErrorCollector,
}

impl<'reg> CompilationContext<'reg> {
    // Delegate to registry for type/function lookup
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.registry.get(hash)
    }

    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.registry.get_function(hash)
    }

    pub fn get_class(&self, hash: TypeHash) -> Option<&ClassEntry> {
        self.registry.get(hash)?.as_class()
    }

    // Register script types during Pass 1
    pub fn register_script_class(&mut self, class: ClassEntry) -> Result<()> {
        self.registry.register_type(class.into())
    }

    pub fn register_script_function(&mut self, func: FunctionEntry) -> Result<()> {
        self.registry.register_function(func)
    }
}
```

### 9.3 Bytecode Storage in FunctionImpl::Script

Bytecode is stored directly in `FunctionImpl::Script` for single-lookup access:

```rust
// FunctionImpl::Script already has unit_id, add bytecode field:
pub enum FunctionImpl {
    Native(Option<NativeFn>),
    Script {
        unit_id: UnitId,
        bytecode: Option<BytecodeChunk>,  // Set after compilation
    },
    Abstract,
    External { module: String },
}

// Workflow:
// 1. Registration pass creates FunctionEntry with bytecode: None
// 2. Compilation pass generates bytecode
// 3. Update: registry.get_function_mut(hash).implementation.set_bytecode(chunk)
// 4. VM execution: registry.get_function(hash).implementation.bytecode()

impl FunctionImpl {
    /// Set bytecode for a script function
    pub fn set_bytecode(&mut self, chunk: BytecodeChunk) {
        if let FunctionImpl::Script { bytecode, .. } = self {
            *bytecode = Some(chunk);
        }
    }

    /// Get bytecode for VM execution
    pub fn bytecode(&self) -> Option<&BytecodeChunk> {
        match self {
            FunctionImpl::Script { bytecode, .. } => bytecode.as_ref(),
            _ => None,
        }
    }
}
```

**Benefits:**
- Single lookup for VM execution
- Hot-reload: just update bytecode in place
- No separate bytecode map to keep in sync

### 9.4 Script Class Fields via PropertyEntry

Use `PropertyEntry` for both virtual properties AND direct fields (no separate FieldEntry needed):

```rust
pub struct PropertyEntry {
    pub name: String,
    pub data_type: DataType,
    pub visibility: Visibility,
    pub getter: Option<TypeHash>,   // Virtual property getter method
    pub setter: Option<TypeHash>,   // Virtual property setter method
}

// Field type determination:
// - Virtual property: getter/setter are Some(hash) -> call methods
// - Direct field: getter/setter are None -> use memory offset

// Offset is computed at compile time based on declaration order in ClassEntry.properties
```

| Field Type | getter | setter | Access Pattern |
|------------|--------|--------|----------------|
| Virtual property (read-only) | `Some(hash)` | `None` | Call `get_x()` |
| Virtual property (read-write) | `Some(hash)` | `Some(hash)` | Call `get_x()` / `set_x()` |
| Direct field | `None` | `None` | Memory offset (computed from position) |

### 9.5 LocalScope

```rust
pub struct LocalScope {
    /// All variables in current scope stack
    variables: FxHashMap<String, Local>,

    /// Scope depth (0 = function params)
    depth: u32,

    /// Stack for restoring shadowed variables
    shadow_stack: Vec<ShadowedVar>,

    /// Next available stack slot
    next_slot: u32,

    /// Captured variables (for lambdas)
    captures: Vec<Capture>,
}

pub struct Local {
    pub name: String,
    pub data_type: DataType,
    pub slot: u32,
    pub depth: u32,
    pub is_const: bool,
    pub is_captured: bool,
}

struct ShadowedVar {
    name: String,
    previous: Option<Local>,
    restore_depth: u32,
}

pub struct Capture {
    pub name: String,
    pub data_type: DataType,
    pub outer_slot: u32,
    pub capture_slot: u32,
}
```

---

## 10. Algorithms

### 10.1 Type Resolution (TypeExpr → DataType)

```
FUNCTION resolve_type(type_expr: TypeExpr, ctx: Context) -> Result<DataType>:
    // 1. Resolve base type name
    base_hash = resolve_type_name(type_expr.name, type_expr.scope, ctx)?

    // 2. Handle template arguments
    IF type_expr.template_args IS NOT empty:
        // Resolve each argument
        resolved_args = []
        FOR arg IN type_expr.template_args:
            resolved_args.push(resolve_type(arg, ctx)?)

        // Check template instance cache
        key = (base_hash, resolved_args.map(|a| a.type_hash))
        IF cached = ctx.template_instances.get(key):
            base_hash = cached
        ELSE:
            // Instantiate template
            base_hash = instantiate_template(base_hash, resolved_args, ctx)?
            ctx.template_instances.insert(key, base_hash)

    // 3. Apply suffixes (handle, array)
    is_handle = false
    FOR suffix IN type_expr.suffixes:
        MATCH suffix:
            Handle: is_handle = true
            Array(size):
                // array<T> is a template type
                base_hash = instantiate_template(ARRAY_TEMPLATE, [DataType::simple(base_hash)], ctx)?

    // 4. Build DataType
    RETURN DataType {
        type_hash: base_hash,
        is_const: type_expr.is_const,
        is_handle,
        is_handle_to_const: type_expr.is_handle_to_const,
        ref_modifier: type_expr.ref_modifier,
    }
```

### 10.2 Name Resolution

```
FUNCTION resolve_type_name(name: String, scope: Option<Scope>, ctx: Context) -> Result<TypeHash>:
    // 1. If explicit scope, use it directly
    IF scope IS Some:
        qualified = scope.path.join("::") + "::" + name
        RETURN ctx.type_by_name.get(qualified)
            .ok_or(Error::UnknownType(qualified))

    // 2. Check for template parameter (hash-based lookup)
    // Template params are registered as TemplateParamEntry with hash = "owner::name"
    IF ctx.current_template_owner IS Some(owner_name):
        param_hash = TypeHash::from_name(format!("{}::{}", owner_name, name))
        IF entry = ctx.registry.get(param_hash):
            IF entry.is_template_param():
                RETURN Ok(param_hash)

    // 3. Check current namespace
    FOR depth IN (0..=ctx.namespace.len()).rev():
        qualified = ctx.namespace[..depth].join("::") + "::" + name
        IF hash = ctx.type_by_name.get(qualified):
            RETURN Ok(hash)

    // 4. Check imported namespaces
    FOR import IN ctx.imports:
        qualified = import.join("::") + "::" + name
        IF hash = ctx.type_by_name.get(qualified):
            RETURN Ok(hash)

    // 5. Check global namespace
    IF hash = ctx.type_by_name.get(name):
        RETURN Ok(hash)

    RETURN Error::UnknownType(name)
```

**Template parameter hash naming convention:**
- `array<T>` parameter T → hash = `"array::T"`
- `dict<K, V>` parameters → `"dict::K"`, `"dict::V"`
- Method template `array<T>::Map<U>` → `"array::Map::U"`

### 10.3 Registration Pass

```
FUNCTION registration_pass(script: Script, ctx: &mut Context) -> Result<()>:
    // Phase 1: Register all type shells (names only)
    FOR item IN script.items:
        MATCH item:
            ClassDecl(class):
                hash = TypeHash::from_name(qualified_name(class.name, ctx))
                ctx.script_types.insert(hash, ScriptType::Class(shell_from_class(class)))
                ctx.type_by_name.insert(qualified_name, hash)

            EnumDecl(enum_):
                // Similar...

            InterfaceDecl(iface):
                // Similar...

            FuncdefDecl(fd):
                // Similar...

            Namespace(ns):
                ctx.namespace.push(ns.name)
                registration_pass(ns.items, ctx)?
                ctx.namespace.pop()

            _: continue

    // Phase 2: Resolve type references (base classes, field types)
    FOR (hash, type_) IN ctx.script_types:
        MATCH type_:
            ScriptType::Class(class):
                IF class.base_class_name IS Some:
                    class.base_class = Some(resolve_type_name(class.base_class_name, ctx)?)
                FOR field IN class.fields:
                    field.data_type = resolve_type(field.type_expr, ctx)?

    // Phase 3: Register functions (with resolved signatures)
    FOR item IN script.items:
        MATCH item:
            FunctionDecl(func):
                register_function(func, None, ctx)?

            ClassDecl(class):
                FOR method IN class.methods:
                    register_function(method, Some(class.hash), ctx)?
                // Auto-generate default constructor if needed
                // Auto-generate opAssign if not explicit

    RETURN Ok(())
```

### 10.4 Function Compilation

```
FUNCTION compile_function(func: ScriptFunction, body: Block, ctx: Context) -> BytecodeChunk:
    // Create emitter
    emitter = BytecodeEmitter::new()

    // Create local scope with parameters
    scope = LocalScope::new()
    FOR param IN func.params:
        scope.declare(param.name, param.data_type, false)

    // Create expression checker
    expr_checker = ExprChecker::new(ctx, &mut scope, &mut emitter, func.owner_type)

    // Compile body
    stmt_compiler = StmtCompiler::new(expr_checker, func.return_type)
    stmt_compiler.compile_block(body)?

    // Ensure return at end
    IF func.return_type.type_hash == VOID:
        emitter.emit(OpCode::ReturnVoid)
    ELSE:
        // If we reach here without return, it's an error (should be caught earlier)
        emitter.emit(OpCode::PushNull)
        emitter.emit(OpCode::Return)

    RETURN emitter.finish()
```

---

## 11. Error Handling

### 11.1 Error Types

```rust
#[derive(Debug, Error)]
pub enum CompilationError {
    // Type errors
    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String, span: Span },

    #[error("unknown type: {name}")]
    UnknownType { name: String, span: Span },

    #[error("cannot convert {from} to {to}")]
    InvalidConversion { from: String, to: String, span: Span },

    // Name errors
    #[error("undefined variable: {name}")]
    UndefinedVariable { name: String, span: Span },

    #[error("undefined function: {name}")]
    UndefinedFunction { name: String, span: Span },

    #[error("duplicate definition: {name}")]
    DuplicateDefinition { name: String, span: Span },

    // Call errors
    #[error("no matching function for call to {name}")]
    NoMatchingFunction { name: String, args: Vec<String>, span: Span },

    #[error("ambiguous call to {name}")]
    AmbiguousCall { name: String, candidates: Vec<String>, span: Span },

    #[error("wrong argument count: expected {expected}, got {got}")]
    WrongArgCount { expected: usize, got: usize, span: Span },

    // Assignment errors
    #[error("cannot assign to {reason}")]
    CannotAssign { reason: String, span: Span },

    #[error("cannot modify const value")]
    ModifyConst { span: Span },

    // Control flow errors
    #[error("break outside loop or switch")]
    BreakOutsideLoop { span: Span },

    #[error("continue outside loop")]
    ContinueOutsideLoop { span: Span },

    #[error("missing return value")]
    MissingReturn { span: Span },

    // Class errors
    #[error("cannot access private member {name}")]
    PrivateAccess { name: String, span: Span },

    #[error("abstract class cannot be instantiated")]
    AbstractInstantiation { name: String, span: Span },
}
```

### 11.2 Error Collection

```rust
pub struct ErrorCollector {
    errors: Vec<CompilationError>,
    warnings: Vec<CompilationWarning>,
    max_errors: usize,
}

impl ErrorCollector {
    pub fn error(&mut self, error: CompilationError) {
        self.errors.push(error);
    }

    pub fn warn(&mut self, warning: CompilationWarning) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn should_abort(&self) -> bool {
        self.errors.len() >= self.max_errors
    }
}
```

### 11.3 Error Recovery

Strategy: Continue compilation after errors to find more issues.

```
// On error, emit a placeholder and continue
FUNCTION handle_error(error: CompilationError, emitter: &mut Emitter) -> ExprInfo:
    collector.error(error)
    // Return "error type" so compilation can continue
    RETURN ExprInfo::rvalue(DataType::error())
```

---

## 12. Testing Strategy

### 12.1 Unit Tests (per component)

**TypeResolver tests:**
```rust
#[test] fn resolve_primitive_types()
#[test] fn resolve_qualified_name()
#[test] fn resolve_template_instance()
#[test] fn resolve_nested_template()
#[test] fn resolve_with_imports()
#[test] fn error_unknown_type()
```

**Conversion tests:**
```rust
#[test] fn identity_conversion()
#[test] fn primitive_widening()
#[test] fn primitive_narrowing()
#[test] fn null_to_handle()
#[test] fn handle_to_const()
#[test] fn derived_to_base()
#[test] fn class_to_interface()
#[test] fn user_defined_constructor()
#[test] fn explicit_only_opcast()
#[test] fn no_conversion_between_unrelated()
```

**Overload tests:**
```rust
#[test] fn exact_match_preferred()
#[test] fn conversion_cost_ranking()
#[test] fn ambiguous_overload_error()
#[test] fn default_params_affect_viability()
#[test] fn variadic_function()
```

**ExprChecker tests:**
```rust
#[test] fn literal_types()
#[test] fn variable_lookup()
#[test] fn binary_arithmetic()
#[test] fn binary_comparison()
#[test] fn operator_overload()
#[test] fn function_call()
#[test] fn method_call()
#[test] fn property_access()
#[test] fn implicit_this()
#[test] fn lambda_inference()
#[test] fn init_list()
```

**StmtCompiler tests:**
```rust
#[test] fn variable_declaration()
#[test] fn auto_type_inference()
#[test] fn if_else_branches()
#[test] fn while_loop()
#[test] fn for_loop()
#[test] fn foreach_loop()
#[test] fn switch_integer()
#[test] fn switch_string()
#[test] fn break_continue()
#[test] fn return_value()
#[test] fn return_void()
```

### 12.2 Integration Tests

Use existing test infrastructure:

```bash
# Parser integration tests (uses test_scripts/*.as)
cargo test --test test_harness

# Module/runtime integration tests
cargo test --test module_tests
```

**Existing test scripts in `test_scripts/`:**
- `hello_world.as`, `literals.as`, `expressions.as`
- `functions.as`, `classes.as`, `inheritance.as`
- `interfaces.as`, `templates.as`, `lambdas.as`
- `properties.as`, `operators.as`, `control_flow.as`
- `performance/*.as` - Performance test scripts

### 12.3 Error Message Tests

```rust
#[test]
fn error_type_mismatch() {
    let result = compile("void main() { int x = \"hello\"; }");
    assert_error!(result, TypeMismatch { expected: "int", got: "string" });
}

#[test]
fn error_undefined_variable() {
    let result = compile("void main() { x = 5; }");
    assert_error!(result, UndefinedVariable { name: "x" });
}
```

---

## 13. Implementation Tasks

### Task Breakdown (20 sessions)

| # | Task | Files | Lines | Tests |
|---|------|-------|-------|-------|
| 1 | ExprInfo + ValueCategory | types/expr_info.rs | ~100 | 10 |
| 2 | Conversion types | types/conversion.rs | ~150 | 5 |
| 3 | Primitive conversions | conversion/primitive.rs | ~250 | 30 |
| 4 | Handle conversions | conversion/handle.rs | ~200 | 20 |
| 5 | User-defined conversions | conversion/user_defined.rs | ~200 | 15 |
| 6 | Type resolution | resolution/type_resolver.rs | ~300 | 25 |
| 7 | Overload resolution | resolution/overload.rs | ~300 | 20 |
| 8 | Operator resolution | resolution/operator.rs | ~200 | 15 |
| 9 | CompilationContext | context.rs | ~400 | 30 |
| 10 | LocalScope | scope.rs | ~250 | 20 |
| 11 | BytecodeChunk + constants | bytecode/chunk.rs | ~150 | 10 |
| 12 | OpCode enum | bytecode/instruction.rs | ~300 | 5 |
| 13 | BytecodeEmitter | bytecode/emitter.rs | ~350 | 25 |
| 14 | Pass 1: Registration | passes/registration.rs | ~450 | 30 |
| 15 | ExprChecker: literals/idents | expr/literals.rs | ~250 | 20 |
| 16 | ExprChecker: operators | expr/operators.rs | ~350 | 25 |
| 17 | ExprChecker: calls | expr/calls.rs | ~400 | 30 |
| 18 | ExprChecker: special | expr/special.rs | ~400 | 25 |
| 19 | StmtCompiler: basic | stmt/basic.rs | ~300 | 25 |
| 20 | StmtCompiler: loops+switch | stmt/loops.rs | ~350 | 25 |
| 21 | Pass 2: Compilation | passes/compilation.rs | ~250 | 15 |
| 22 | Integration + lib.rs | lib.rs + tests | ~150 | 50 |

**Total: ~5600 lines, ~470 tests**

---

## File Structure

```
crates/angelscript-compiler/src/
├── lib.rs                      # Public API
├── context.rs                  # CompilationContext
├── scope.rs                    # LocalScope
├── types/
│   ├── mod.rs
│   ├── expr_info.rs            # ExprInfo, ValueCategory
│   └── conversion.rs           # Conversion, ConversionKind
├── conversion/
│   ├── mod.rs                  # ConversionChecker trait
│   ├── primitive.rs            # Primitive conversions
│   ├── handle.rs               # Handle conversions
│   └── user_defined.rs         # Constructor, opConv, opCast
├── resolution/
│   ├── mod.rs
│   ├── type_resolver.rs        # TypeExpr → DataType
│   ├── overload.rs             # Function overload resolution
│   └── operator.rs             # Operator resolution
├── expr/
│   ├── mod.rs                  # ExprChecker struct
│   ├── literals.rs             # Literals, identifiers
│   ├── operators.rs            # Binary, unary, assignment
│   ├── calls.rs                # Function, method, constructor calls
│   └── special.rs              # Cast, lambda, init list
├── stmt/
│   ├── mod.rs                  # StmtCompiler struct
│   ├── basic.rs                # Block, var decl, return, if, while
│   └── loops.rs                # For, foreach, switch, break, continue
├── bytecode/
│   ├── mod.rs
│   ├── chunk.rs                # BytecodeChunk, Constant
│   ├── instruction.rs          # OpCode enum
│   └── emitter.rs              # BytecodeEmitter
└── passes/
    ├── mod.rs
    ├── registration.rs         # Pass 1
    └── compilation.rs          # Pass 2

tests/
└── compiler_tests.rs           # Integration tests
```

---

## Important Notes

- **Fresh design**: Do not copy old compiler patterns
- **File size limit**: ≤500 lines per file
- **Test coverage**: Each component has dedicated tests
- **Performance**: O(1) lookups via TypeHash
- **Error recovery**: Continue after errors to find more issues
- **Bidirectional typing**: Use check mode when expected type is known

---

## 14. Performance Considerations

### 14.1 Compilation Performance

| Strategy | Implementation | Impact |
|----------|----------------|--------|
| **O(1) type lookups** | `FxHashMap<TypeHash, _>` for all type/function maps | Eliminates string comparisons |
| **TypeHash as Copy** | 64-bit value, no allocation | Zero-cost passing, no Arc/Rc |
| **DataType as Copy** | 16-byte struct, stack-allocated | No heap allocation for types |
| **Single-pass name resolution** | Cache qualified names during registration | No repeated string concatenation |
| **Lazy template instantiation** | Only instantiate when used | Avoid unused template expansion |
| **Interned strings** | String pool for identifiers | Reduce memory, faster comparison |

### 14.2 Bytecode Performance

| Strategy | Implementation | Impact |
|----------|----------------|--------|
| **Dense instruction encoding** | Single-byte opcodes where possible | Better cache utilization |
| **Type-specific opcodes** | `AddI32`, `AddF64` instead of generic `Add` | No runtime type dispatch |
| **Constant pool** | Deduplicate constants | Smaller bytecode size |
| **Jump offset as i16** | Two-byte relative jumps | Covers 99% of cases |
| **Stack-based VM** | Simple push/pop operations | Predictable memory access |
| **Inline small constants** | `PushTrue`, `PushFalse`, `PushNull` | Avoid constant pool lookup |

### 14.3 Memory Efficiency

```rust
// Type sizes (verify with static_assert)
const _: () = assert!(std::mem::size_of::<TypeHash>() == 8);
const _: () = assert!(std::mem::size_of::<DataType>() == 16);  // or less
const _: () = assert!(std::mem::size_of::<ExprInfo>() == 24);  // DataType + ValueCategory

// Use FxHashMap (rustc-hash) for hot paths
use rustc_hash::FxHashMap;

// Pre-allocate vectors where size is known
let mut constants = Vec::with_capacity(estimated_count);
```

### 14.4 Avoiding Common Pitfalls

| Pitfall | Solution |
|---------|----------|
| `format!()` in hot paths | Use pre-computed TypeHash |
| String concatenation for names | Intern strings, cache qualified names |
| Cloning large types | Make types Copy where possible |
| HashMap with String keys | Use TypeHash keys |
| Repeated type resolution | Cache in CompilationContext |
| Linear search for overloads | Index by name, then filter |
| Heap allocation per expression | Arena allocator for temporaries |
| Vec growth during emission | Pre-size vectors based on AST size |
| Branch misprediction in type checks | Match on discriminant, predictable patterns |

### 14.4.1 Critical Path Optimizations (for <1ms/1000 lines)

```rust
// 1. Single-pass AST traversal (no multiple walks)
// Register types AND collect function bodies in one pass

// 2. Arena allocation for compilation temporaries
pub struct CompileArena {
    exprs: TypedArena<ExprInfo>,
    locals: TypedArena<Local>,
}

// 3. Inline hot functions
#[inline(always)]
fn is_primitive(hash: TypeHash) -> bool {
    hash.0 <= primitives::DOUBLE.0  // Primitives have low hash values
}

// 4. Avoid Option<T> unwrapping in hot paths
// Use sentinel values or unsafe unchecked access where safe

// 5. Pre-compute conversion tables at startup
static PRIMITIVE_CONV_COST: [[u8; 12]; 12] = [...];

// 6. Batch bytecode emission
impl BytecodeEmitter {
    // Emit multiple instructions at once
    pub fn emit_batch(&mut self, ops: &[OpCode]) {
        self.code.extend(ops.iter().map(|op| *op as u8));
    }
}

// 7. Avoid allocations in ExprChecker
impl ExprChecker<'_> {
    // Return Copy type, not allocated
    fn infer(&mut self, expr: &Expr) -> Result<ExprInfo, ()> {
        // ExprInfo is Copy - no allocation
    }
}
```

### 14.5 Benchmarking Strategy

Use existing benchmark infrastructure in `benches/module_benchmarks.rs`:

```bash
# Run compiler-related benchmarks
cargo bench -- "unit/file_sizes"      # Scales from tiny to 5000 lines
cargo bench -- "unit/features"        # Functions, classes, expressions
cargo bench -- "unit/real_world"      # Game logic, utilities
cargo bench -- "unit/complexity"      # Nesting, branching

# Profile with puffin
cargo bench --features profile-with-puffin
```

**Performance Targets:**
| Benchmark | Target |
|-----------|--------|
| 1000-line script | < 1ms |
| 5000-line script | < 3ms |
| Type lookup | < 50ns |
| Overload resolution (5 candidates) | < 200ns |
| Empty function baseline | < 1μs |

### 14.6 Profiling Integration

```rust
// Optional puffin profiling (feature-gated)
#[cfg(feature = "profile")]
puffin::profile_scope!("registration_pass");

// Key areas to profile:
// 1. Type resolution (resolve_type)
// 2. Overload resolution (resolve_overload)
// 3. Bytecode emission (emit)
// 4. Conversion checking (can_convert)
```

---

## 15. Source Spans and Location Tracking

### 15.1 Span Structure

```rust
/// Byte offset range in source file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,  // Byte offset of first character
    pub end: u32,    // Byte offset after last character
}

/// Full source location with file info
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file_id: FileId,
    pub span: Span,
}

/// Lazily computed line/column (only for error display)
#[derive(Debug, Clone)]
pub struct LineColumn {
    pub line: u32,      // 1-indexed
    pub column: u32,    // 1-indexed, in characters (not bytes)
}

impl Span {
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn to_line_column(&self, source: &str) -> (LineColumn, LineColumn) {
        // Compute lazily only when displaying errors
    }
}
```

### 15.2 Span Propagation

**AST nodes carry spans:**
```rust
pub struct Expr<'ast> {
    pub kind: ExprKind<'ast>,
    pub span: Span,
}

pub struct Stmt<'ast> {
    pub kind: StmtKind<'ast>,
    pub span: Span,
}
```

**Compiler passes preserve spans:**
```rust
impl ExprChecker {
    fn infer(&mut self, expr: &Expr) -> Result<ExprInfo, CompileError> {
        match &expr.kind {
            ExprKind::Binary { left, op, right } => {
                let left_info = self.infer(left)?;
                let right_info = self.infer(right)?;

                let result = self.resolve_binary_op(left_info, *op, right_info)
                    .map_err(|e| e.with_span(expr.span))?;  // Attach span to error

                Ok(result)
            }
            // ...
        }
    }
}
```

**Bytecode retains line info:**
```rust
pub struct BytecodeChunk {
    pub code: Vec<u8>,
    pub constants: Vec<Constant>,

    /// Line number for each byte offset (run-length encoded)
    pub line_info: LineInfo,
}

pub struct LineInfo {
    /// (byte_offset, line_number) pairs, sorted by offset
    entries: Vec<(u32, u32)>,
}

impl LineInfo {
    pub fn get_line(&self, offset: usize) -> u32 {
        // Binary search for the entry covering this offset
    }
}
```

### 15.3 Error Display with Spans

```rust
impl CompileError {
    pub fn display(&self, sources: &SourceMap) -> String {
        let loc = sources.lookup(self.span);
        let (start_lc, end_lc) = self.span.to_line_column(&loc.source);

        format!(
            "error[E{:04}]: {}\n  --> {}:{}:{}\n   |\n{} | {}\n   | {}\n",
            self.code(),
            self.message(),
            loc.filename,
            start_lc.line,
            start_lc.column,
            start_lc.line,
            loc.line_text(start_lc.line),
            self.underline(start_lc.column, end_lc.column),
        )
    }
}

// Example output:
// error[E0001]: type mismatch: expected int, got string
//   --> script.as:10:5
//    |
// 10 |     int x = "hello";
//    |             ^^^^^^^
```

---

## 16. Debug Tracing Infrastructure

### 16.1 Compilation Tracing

```rust
/// Compile-time tracing (feature-gated)
#[cfg(feature = "trace")]
macro_rules! trace_compile {
    ($($arg:tt)*) => {
        eprintln!("[COMPILE] {}", format!($($arg)*));
    };
}

#[cfg(not(feature = "trace"))]
macro_rules! trace_compile {
    ($($arg:tt)*) => {};
}

// Usage in compiler:
impl ExprChecker {
    fn infer(&mut self, expr: &Expr) -> Result<ExprInfo> {
        trace_compile!("infer {:?} at {:?}", expr.kind.name(), expr.span);
        // ...
        trace_compile!("  -> {:?}", result);
        result
    }
}
```

### 16.2 Runtime Tracing (for VM)

```rust
/// Bytecode execution trace
#[cfg(feature = "trace-execution")]
pub struct ExecutionTracer {
    pub enabled: bool,
    pub depth: usize,
    pub output: Box<dyn Write>,
}

impl ExecutionTracer {
    pub fn trace_instruction(&mut self, ip: usize, op: OpCode, chunk: &BytecodeChunk) {
        if !self.enabled { return; }

        let line = chunk.line_info.get_line(ip);
        writeln!(
            self.output,
            "{:04} {:4} | {:?}",
            ip, line, op
        ).ok();
    }

    pub fn trace_call(&mut self, func_name: &str) {
        if !self.enabled { return; }

        writeln!(self.output, "{}-> {}", "  ".repeat(self.depth), func_name).ok();
        self.depth += 1;
    }

    pub fn trace_return(&mut self) {
        if !self.enabled { return; }

        self.depth = self.depth.saturating_sub(1);
        writeln!(self.output, "{}<-", "  ".repeat(self.depth)).ok();
    }
}
```

### 16.3 Debug Information Generation

```rust
/// Debug info for compiled functions (for debugger integration)
pub struct DebugInfo {
    /// Function hash -> debug data
    pub functions: FxHashMap<TypeHash, FunctionDebugInfo>,
}

pub struct FunctionDebugInfo {
    pub name: String,
    pub file_id: FileId,
    pub span: Span,

    /// Local variable info for debugger
    pub locals: Vec<LocalDebugInfo>,

    /// Bytecode offset -> source span mapping
    pub source_map: Vec<(u32, Span)>,
}

pub struct LocalDebugInfo {
    pub name: String,
    pub slot: u32,
    pub data_type: DataType,
    pub scope_start: u32,  // Bytecode offset where var becomes valid
    pub scope_end: u32,    // Bytecode offset where var goes out of scope
}
```

### 16.4 Structured Logging

```rust
/// Structured compile events (for tooling integration)
#[derive(Debug, Serialize)]
pub enum CompileEvent {
    PassStart { name: &'static str },
    PassEnd { name: &'static str, duration_us: u64 },

    TypeRegistered { hash: TypeHash, name: String },
    FunctionCompiled { hash: TypeHash, name: String, bytecode_size: usize },

    Error { code: u32, message: String, span: Span },
    Warning { code: u32, message: String, span: Span },
}

pub trait CompileListener {
    fn on_event(&mut self, event: CompileEvent);
}

// Usage:
pub struct Compiler<L: CompileListener> {
    listener: L,
}

impl<L: CompileListener> Compiler<L> {
    fn compile_function(&mut self, func: &ScriptFunction) {
        self.listener.on_event(CompileEvent::PassStart { name: "compile_function" });
        let start = Instant::now();

        // ... compilation ...

        self.listener.on_event(CompileEvent::FunctionCompiled {
            hash: func.hash,
            name: func.name.clone(),
            bytecode_size: chunk.code.len(),
        });
        self.listener.on_event(CompileEvent::PassEnd {
            name: "compile_function",
            duration_us: start.elapsed().as_micros() as u64,
        });
    }
}
```

---

## 17. Template Instantiation

This section details how the compiler handles template instantiation for types, functions, and child funcdefs.

### 17.1 Template Components Summary

| Component | Registry Storage | Hash Pattern |
|-----------|------------------|--------------|
| Template Type | `ClassEntry` with `template_params: Vec<TypeHash>` | `from_name(name)` |
| Template Param | `TemplateParamEntry` | `owner::name` (e.g., `array::T`) |
| Template Instance | `ClassEntry` with `template: Some(hash)` + `type_args` | `from_template_instance(template, args)` |
| Template Function | `FunctionDef` with `template_params: Vec<TypeHash>` | `from_name(qualified)` |
| Child Funcdef | `FuncdefEntry` with `parent_type: Some(hash)` | `from_name(qualified)` |
| FFI Specialization | Pre-registered `ClassEntry` | Same as template instance |

### 17.2 Template Instance Cache

The `TypeRegistry` maintains a cache for O(1) template instance lookup:

```rust
pub struct TypeRegistry {
    // ... existing fields ...

    /// Pre-computed template instance lookups.
    /// Maps (template_hash, arg_hashes) → instance_hash
    /// Pre-populated with FFI specializations, updated during compilation.
    template_instance_cache: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,
}

impl TypeRegistry {
    /// Register a template instance in the cache.
    pub fn cache_template_instance(
        &mut self,
        template: TypeHash,
        args: Vec<TypeHash>,
        instance: TypeHash,
    ) {
        self.template_instance_cache.insert((template, args), instance);
    }

    /// Look up a cached template instance.
    pub fn get_cached_instance(
        &self,
        template: TypeHash,
        args: &[TypeHash],
    ) -> Option<TypeHash> {
        self.template_instance_cache.get(&(template, args.to_vec())).copied()
    }
}
```

**Invariant**: If a template instance exists in the registry, it MUST be in the cache.

### 17.3 FFI Specialization Pre-Registration

FFI specializations are pre-registered during module installation:

```rust
// During Context::install_class for specializations:
fn install_class(meta: ClassMeta, namespace: &[String]) -> Result<()> {
    if let Some(template_name) = meta.specialization_of {
        let template_hash = TypeHash::from_name(template_name);
        let specialization_hash = TypeHash::from_template_instance(
            template_hash,
            &meta.specialization_args,
        );

        // Convert TypeHash args to DataType args
        let type_args: Vec<DataType> = meta.specialization_args
            .iter()
            .map(|h| DataType::simple(*h))
            .collect();

        // Create ClassEntry with template instance info
        let entry = ClassEntry::new(
            meta.name,
            qualified_name,
            specialization_hash,
            meta.type_kind,
            TypeSource::ffi_typed::<T>(),
        )
        .with_template_instance(template_hash, type_args);

        registry.register_type(entry.into())?;

        // Pre-populate cache for fast lookup
        registry.cache_template_instance(
            template_hash,
            meta.specialization_args.clone(),
            specialization_hash,
        );
    }
    Ok(())
}
```

### 17.4 Type Substitution Map

Template instantiation builds a substitution map from template params to concrete types:

```rust
/// Build substitution map for template instantiation.
fn build_substitution_map(
    template_params: &[TypeHash],
    type_args: &[DataType],
) -> FxHashMap<TypeHash, DataType> {
    template_params
        .iter()
        .zip(type_args.iter())
        .map(|(param_hash, arg)| (*param_hash, *arg))
        .collect()
}

/// Substitute a type through the map.
fn substitute_type(
    data_type: DataType,
    subst_map: &FxHashMap<TypeHash, DataType>,
    ctx: &mut CompilationContext,
) -> Result<DataType> {
    // Check if this is a template parameter
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        // Apply modifiers from original to replacement
        return Ok(DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const || replacement.is_handle_to_const,
            ref_modifier: data_type.ref_modifier,
        });
    }

    // Check if this is a template instance that needs recursive substitution
    if let Some(class) = ctx.get_class(data_type.type_hash) {
        if class.is_template_instance() {
            let new_args: Vec<DataType> = class.type_args
                .iter()
                .map(|arg| substitute_type(*arg, subst_map, ctx))
                .collect::<Result<_>>()?;

            let new_hash = find_or_instantiate_template(
                class.template.unwrap(),
                new_args,
                ctx,
            )?;
            return Ok(data_type.with_type_hash(new_hash));
        }
    }

    // Not a template param, return unchanged
    Ok(data_type)
}
```

### 17.5 Template Type Instantiation

The main entry point for instantiating template types:

```rust
/// Find or instantiate a template type.
///
/// This is the main entry point called when the compiler encounters
/// a template type expression like `array<int>`.
fn find_or_instantiate_template(
    template_hash: TypeHash,
    type_args: Vec<DataType>,
    ctx: &mut CompilationContext,
) -> Result<TypeHash> {
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();

    // 1. Check cache - includes FFI specializations and previous instantiations
    if let Some(instance_hash) = ctx.registry.get_cached_instance(template_hash, &arg_hashes) {
        return Ok(instance_hash);
    }

    // 2. Cache miss - instantiate and cache
    let instance_hash = instantiate_template_type(template_hash, type_args.clone(), ctx)?;
    ctx.registry.cache_template_instance(template_hash, arg_hashes, instance_hash);
    Ok(instance_hash)
}

/// Instantiate a template type with concrete type arguments.
fn instantiate_template_type(
    template_hash: TypeHash,
    type_args: Vec<DataType>,
    ctx: &mut CompilationContext,
) -> Result<TypeHash> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(template_hash, &arg_hashes);

    // 2. Get template definition
    let template = ctx.get_class(template_hash)
        .ok_or_else(|| CompileError::TypeNotFound(template_hash))?
        .clone();  // Clone to release borrow

    assert!(template.is_template(), "Expected template type");

    // 3. Validate via callback (if registered)
    if ctx.registry.has_template_callback(template_hash) {
        let info = TemplateInstanceInfo::new(&template.name, type_args.clone());
        let validation = ctx.registry.validate_template_instance(&info);
        if !validation.is_valid {
            return Err(CompileError::TemplateValidationFailed {
                template: template.name.clone(),
                args: type_args,
                message: validation.error.unwrap_or_default(),
            });
        }
    }

    // 4. Build substitution map
    let subst_map = build_substitution_map(&template.template_params, &type_args);

    // 5. Create instance entry
    let instance_name = format_template_instance_name(&template.name, &type_args);
    let mut instance = ClassEntry::new(
        instance_name.clone(),
        instance_name,
        instance_hash,
        template.type_kind,
        TypeSource::script(ctx.unit_id, Span::default()),
    )
    .with_template_instance(template_hash, type_args);

    // 6. Substitute base class
    if let Some(base) = template.base_class {
        let base_type = DataType::simple(base);
        let subst_base = substitute_type(base_type, &subst_map, ctx)?;
        instance = instance.with_base(subst_base.type_hash);
    }

    // 7. Instantiate methods
    for method_hash in &template.methods {
        let inst_method = instantiate_method(*method_hash, &subst_map, instance_hash, ctx)?;
        instance.methods.push(inst_method);
    }

    // 8. Instantiate properties
    for prop in &template.properties {
        let inst_prop = PropertyEntry {
            name: prop.name.clone(),
            data_type: substitute_type(prop.data_type, &subst_map, ctx)?,
            visibility: prop.visibility,
            getter: prop.getter,  // Getter/setter hashes may need instantiation too
            setter: prop.setter,
        };
        instance.properties.push(inst_prop);
    }

    // 9. Register instance
    ctx.registry.register_type(instance.into())?;

    Ok(instance_hash)
}
```

### 17.6 Template Function Instantiation

There are three cases for template functions:

| Case | template_params | object_type | Params Reference |
|------|-----------------|-------------|------------------|
| Method of template type | Empty | `Some(template)` | Parent's params |
| Standalone template function | Own params | `None` | Own params |
| Template method (mixed) | Own params | `Some(template)` | Both |

```rust
/// Instantiate a standalone template function.
fn instantiate_template_function(
    func_hash: TypeHash,
    type_args: Vec<DataType>,
    ctx: &mut CompilationContext,
) -> Result<TypeHash> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_function_instance(func_hash, &arg_hashes);

    // 2. Check cache
    if ctx.registry.get_function(instance_hash).is_some() {
        return Ok(instance_hash);
    }

    // 3. Get template definition
    let template = ctx.registry.get_function(func_hash)
        .ok_or_else(|| CompileError::FunctionNotFound(func_hash))?
        .clone();

    // 4. Build substitution map
    let subst_map = build_substitution_map(&template.def.template_params, &type_args);

    // 5. Substitute param types and return type
    let inst_params: Vec<Param> = template.def.params
        .iter()
        .map(|p| Ok(Param {
            name: p.name.clone(),
            data_type: substitute_type(p.data_type, &subst_map, ctx)?,
            default_value: p.default_value.clone(),
        }))
        .collect::<Result<_>>()?;

    let inst_return = substitute_type(template.def.return_type, &subst_map, ctx)?;

    // 6. Create instance definition
    let inst_name = format!("{}<{}>", template.def.name, format_type_args(&type_args));
    let inst_def = FunctionDef::new(
        instance_hash,
        inst_name,
        template.def.namespace.clone(),
        inst_params,
        inst_return,
        template.def.object_type,
        template.def.traits.clone(),
        true,  // is_script
        template.def.visibility,
    );

    // 7. Handle implementation
    let inst_impl = match &template.implementation {
        FunctionImpl::Native(native) => FunctionImpl::Native(native.clone()),
        FunctionImpl::Script { unit_id, .. } => FunctionImpl::Script {
            unit_id: *unit_id,
            bytecode: None,  // Will be generated during compilation
        },
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, template.source.clone());
    ctx.registry.register_function(inst_entry)?;

    Ok(instance_hash)
}

/// Instantiate a method for a template type instance.
fn instantiate_method(
    method_hash: TypeHash,
    parent_subst_map: &FxHashMap<TypeHash, DataType>,
    parent_instance_hash: TypeHash,
    ctx: &mut CompilationContext,
) -> Result<TypeHash> {
    let method = ctx.registry.get_function(method_hash)
        .ok_or_else(|| CompileError::FunctionNotFound(method_hash))?
        .clone();

    // Substitute param types
    let inst_params: Vec<Param> = method.def.params
        .iter()
        .map(|p| Ok(Param {
            name: p.name.clone(),
            data_type: substitute_type(p.data_type, parent_subst_map, ctx)?,
            default_value: p.default_value.clone(),
        }))
        .collect::<Result<_>>()?;

    let inst_return = substitute_type(method.def.return_type, parent_subst_map, ctx)?;

    // Compute instance method hash
    let param_hashes: Vec<TypeHash> = inst_params.iter().map(|p| p.data_type.type_hash).collect();
    let instance_hash = TypeHash::from_method(
        parent_instance_hash,
        &method.def.name,
        &param_hashes,
        method.def.traits.is_const,
    );

    // Check if already exists
    if ctx.registry.get_function(instance_hash).is_some() {
        return Ok(instance_hash);
    }

    // Create instance
    let inst_def = FunctionDef::new(
        instance_hash,
        method.def.name.clone(),
        method.def.namespace.clone(),
        inst_params,
        inst_return,
        Some(parent_instance_hash),  // Object type is the instance
        method.def.traits.clone(),
        true,
        method.def.visibility,
    );

    let inst_impl = match &method.implementation {
        FunctionImpl::Native(native) => FunctionImpl::Native(native.clone()),
        FunctionImpl::Script { unit_id, .. } => FunctionImpl::Script {
            unit_id: *unit_id,
            bytecode: None,
        },
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, method.source.clone());
    ctx.registry.register_function(inst_entry)?;

    Ok(instance_hash)
}
```

### 17.7 Child Funcdef Instantiation

Funcdefs that belong to template types (e.g., `array<T>::Callback`):

```rust
/// Instantiate a child funcdef for a template type instance.
fn instantiate_child_funcdef(
    funcdef_hash: TypeHash,
    parent_type_args: Vec<DataType>,
    ctx: &mut CompilationContext,
) -> Result<TypeHash> {
    let funcdef = ctx.registry.get(funcdef_hash)
        .and_then(|e| e.as_funcdef())
        .ok_or_else(|| CompileError::TypeNotFound(funcdef_hash))?
        .clone();

    let parent_hash = funcdef.parent_type
        .expect("Child funcdef must have parent");

    // Get parent template's param hashes
    let parent_template = ctx.get_class(parent_hash)
        .ok_or_else(|| CompileError::TypeNotFound(parent_hash))?
        .clone();

    // Build substitution map
    let subst_map = build_substitution_map(&parent_template.template_params, &parent_type_args);

    // Compute instance hash
    let arg_hashes: Vec<TypeHash> = parent_type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(funcdef_hash, &arg_hashes);

    // Check cache
    if ctx.registry.get(instance_hash).is_some() {
        return Ok(instance_hash);
    }

    // Substitute params and return type
    let inst_params: Vec<DataType> = funcdef.params
        .iter()
        .map(|p| substitute_type(*p, &subst_map, ctx))
        .collect::<Result<_>>()?;

    let inst_return = substitute_type(funcdef.return_type, &subst_map, ctx)?;

    // Create instance name (e.g., "array<int>::Callback")
    let parent_instance = find_or_instantiate_template(parent_hash, parent_type_args.clone(), ctx)?;
    let parent_instance_name = ctx.get_class(parent_instance)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let inst_name = format!("{}::{}", parent_instance_name, funcdef.name);

    let inst_entry = FuncdefEntry::new_child(
        funcdef.name.clone(),
        inst_name,
        instance_hash,
        TypeSource::script(ctx.unit_id, Span::default()),
        inst_params,
        inst_return,
        parent_instance,
    );

    ctx.registry.register_type(inst_entry.into())?;
    Ok(instance_hash)
}
```

### 17.8 if_handle_then_const Feature

For template parameters that should propagate const to handle targets:

```rust
/// Substitute type with if_handle_then_const flag.
fn substitute_type_with_flags(
    data_type: DataType,
    subst_map: &FxHashMap<TypeHash, DataType>,
    if_handle_then_const: bool,
    ctx: &mut CompilationContext,
) -> Result<DataType> {
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        return Ok(DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const
                || replacement.is_handle_to_const
                || (if_handle_then_const && replacement.is_handle && data_type.is_const),
            ref_modifier: data_type.ref_modifier,
        });
    }
    substitute_type(data_type, subst_map, ctx)
}
```

**Behavior:**
- `const T&in` with `T=Obj@` and `if_handle_then_const=true`:
  - Result: `const Obj@ const &in` (both handle and target are const)
- Without flag:
  - Result: `Obj@ const &in` (only handle is const)

### 17.9 Helper Functions

```rust
/// Format template instance name (e.g., "array<int>" or "dict<string, int>").
fn format_template_instance_name(template_name: &str, type_args: &[DataType]) -> String {
    let args_str = type_args
        .iter()
        .map(|a| format_type_name(a))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}<{}>", template_name, args_str)
}

/// Format type arguments for display.
fn format_type_args(type_args: &[DataType]) -> String {
    type_args
        .iter()
        .map(|a| format_type_name(a))
        .collect::<Vec<_>>()
        .join(", ")
}
```

### 17.10 Instantiation Triggers

Template instantiation is triggered by:

1. **Type expression with template arguments**: `array<int>`
2. **Variable declaration with template type**: `array<int> arr;`
3. **Function parameter/return with template type**: `void foo(array<int>@ arr)`
4. **Cast expression**: `cast<array<int>>(x)`
5. **Constructor call**: `array<int>()`
6. **Template function call with explicit args**: `identity<int>(42)`
7. **Child funcdef access**: `array<int>::Callback`

### 17.11 Example Flow

```
FFI Registration:
  Module.ty::<IntArray>()
  → ClassMeta { specialization_of: Some("array"), specialization_args: [INT32] }
  → install_class:
      1. computes hash = from_template_instance(array, [INT32])
      2. registry.types[hash] = ClassEntry { name: "array<int>", ... }
      3. registry.cache_template_instance(array, [INT32], hash)

Script Compilation:
  "array<int> arr;"
  → resolve type "array<int>"
  → template_hash = lookup("array")
  → type_args = [DataType::simple(INT32)]
  → arg_hashes = [INT32]
  → find_or_instantiate_template(array, [DataType::simple(INT32)])
  → registry.get_cached_instance(array, [INT32]) == Some(hash)  // Cache hit!
  → return hash  // Uses optimized IntArray
```
