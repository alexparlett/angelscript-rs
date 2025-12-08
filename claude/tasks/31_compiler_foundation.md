# Task 31: Compiler Foundation

## Overview

Establish the core types and infrastructure for the AngelScript compiler. This is the foundation all other compiler phases build upon.

## Goals

1. Create the `angelscript-compiler` crate structure
2. Define core types: `ExprInfo`, `Conversion`, `BytecodeChunk`, `OpCode`
3. Define compiler error types
4. Set up the module structure

## Dependencies

- `angelscript-core` (TypeHash, DataType, etc.)
- `angelscript-parser` (AST types)
- `angelscript-registry` (TypeRegistry)

## Files to Create

```
crates/angelscript-compiler/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs           # CompileError, Result type
│   ├── expr_info.rs       # ExprInfo (expression type result)
│   ├── conversion.rs      # Conversion, ConversionKind
│   └── bytecode/
│       ├── mod.rs
│       ├── opcode.rs      # OpCode enum
│       ├── chunk.rs       # BytecodeChunk
│       └── constant.rs    # Constant pool types
```

## Detailed Implementation

### 1. Cargo.toml

```toml
[package]
name = "angelscript-compiler"
version = "0.1.0"
edition = "2021"

[dependencies]
angelscript-core = { path = "../angelscript-core" }
angelscript-parser = { path = "../angelscript-parser" }
angelscript-registry = { path = "../angelscript-registry" }
thiserror = "1.0"
rustc-hash = "1.1"
```

### 2. Error Types (error.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompileError>;

#[derive(Debug, Error)]
pub enum CompileError {
    // Type errors
    #[error("type not found: {name}")]
    TypeNotFound { name: String, span: Span },

    #[error("cannot convert {from} to {to}")]
    ConversionError { from: String, to: String, span: Span },

    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String, span: Span },

    // Function errors
    #[error("function not found: {name}")]
    FunctionNotFound { name: String, span: Span },

    #[error("no matching overload for {name} with arguments ({args})")]
    NoMatchingOverload { name: String, args: String, span: Span },

    #[error("ambiguous overload for {name}")]
    AmbiguousOverload { name: String, candidates: Vec<String>, span: Span },

    #[error("wrong number of arguments: expected {expected}, got {got}")]
    WrongArgCount { expected: usize, got: usize, span: Span },

    // Variable errors
    #[error("undefined variable: {name}")]
    UndefinedVariable { name: String, span: Span },

    #[error("variable already defined: {name}")]
    VariableRedefinition { name: String, span: Span },

    #[error("cannot assign to {reason}")]
    NotAssignable { reason: String, span: Span },

    #[error("cannot modify const value")]
    ConstViolation { span: Span },

    // Control flow errors
    #[error("break outside of loop")]
    BreakOutsideLoop { span: Span },

    #[error("continue outside of loop")]
    ContinueOutsideLoop { span: Span },

    #[error("return type mismatch: expected {expected}, got {got}")]
    ReturnTypeMismatch { expected: String, got: String, span: Span },

    // Template errors
    #[error("template validation failed for {template}<{args}>: {message}")]
    TemplateValidationFailed { template: String, args: String, message: String, span: Span },

    #[error("wrong number of template arguments: expected {expected}, got {got}")]
    WrongTemplateArgCount { expected: usize, got: usize, span: Span },

    // Member access errors
    #[error("no member {member} on type {type_name}")]
    MemberNotFound { type_name: String, member: String, span: Span },

    #[error("cannot access private member {member}")]
    PrivateMemberAccess { member: String, span: Span },

    // Operator errors
    #[error("no operator {op} for types {left} and {right}")]
    NoOperator { op: String, left: String, right: String, span: Span },

    // Other
    #[error("internal compiler error: {message}")]
    Internal { message: String },
}

impl CompileError {
    pub fn span(&self) -> Option<Span> {
        // Return span for error reporting
        match self {
            Self::TypeNotFound { span, .. } => Some(*span),
            Self::ConversionError { span, .. } => Some(*span),
            Self::TypeMismatch { span, .. } => Some(*span),
            // ... etc
            Self::Internal { .. } => None,
        }
    }
}
```

### 3. ExprInfo (expr_info.rs)

```rust
use angelscript_core::DataType;

/// Result of type-checking an expression.
/// Contains the type and lvalue/mutability information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExprInfo {
    /// The type of the expression
    pub data_type: DataType,
    /// Whether this is an lvalue (can appear on left side of assignment)
    pub is_lvalue: bool,
    /// Whether this lvalue can be modified (false for const)
    pub is_mutable: bool,
}

impl ExprInfo {
    /// Create an rvalue (temporary, cannot be assigned to)
    pub fn rvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: false,
            is_mutable: false,
        }
    }

    /// Create a mutable lvalue (can be assigned to)
    pub fn lvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: true,
        }
    }

    /// Create a const lvalue (can be read but not assigned)
    pub fn const_lvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: false,
        }
    }

    /// Check if this can be assigned to
    pub fn is_assignable(&self) -> bool {
        self.is_lvalue && self.is_mutable
    }

    /// Convert to rvalue (for reading)
    pub fn to_rvalue(self) -> Self {
        Self {
            data_type: self.data_type,
            is_lvalue: false,
            is_mutable: false,
        }
    }
}
```

### 4. Conversion Types (conversion.rs)

```rust
use angelscript_core::TypeHash;

/// A type conversion with its cost for overload resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub kind: ConversionKind,
    pub cost: u32,
    pub is_implicit: bool,
}

/// The kind of conversion being performed.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// No conversion needed (exact match)
    Identity,

    /// Primitive type conversion (int -> float, etc.)
    Primitive { from: TypeHash, to: TypeHash },

    /// Null literal to handle type
    NullToHandle,

    /// Handle to const handle
    HandleToConst,

    /// Derived class to base class
    DerivedToBase { base: TypeHash },

    /// Class to interface it implements
    ClassToInterface { interface: TypeHash },

    /// Implicit conversion via constructor
    ConstructorConversion { constructor: TypeHash },

    /// Implicit conversion via opImplConv method
    ImplicitConvMethod { method: TypeHash },

    /// Explicit cast via opCast method
    ExplicitCastMethod { method: TypeHash },

    /// Value type to handle (@value)
    ValueToHandle,

    /// Enum to underlying integer type
    EnumToInt,

    /// Integer to enum type
    IntToEnum { enum_type: TypeHash },
}

impl Conversion {
    pub const COST_EXACT: u32 = 0;
    pub const COST_CONST_ADDITION: u32 = 1;
    pub const COST_PRIMITIVE_WIDENING: u32 = 2;
    pub const COST_PRIMITIVE_NARROWING: u32 = 4;
    pub const COST_DERIVED_TO_BASE: u32 = 5;
    pub const COST_CLASS_TO_INTERFACE: u32 = 6;
    pub const COST_USER_IMPLICIT: u32 = 10;
    pub const COST_EXPLICIT_ONLY: u32 = 100;

    /// Create an identity conversion (no conversion needed)
    pub fn identity() -> Self {
        Self {
            kind: ConversionKind::Identity,
            cost: Self::COST_EXACT,
            is_implicit: true,
        }
    }

    /// Check if this conversion can be used implicitly
    pub fn is_implicit(&self) -> bool {
        self.is_implicit
    }

    /// Check if this is an exact match (no conversion)
    pub fn is_exact(&self) -> bool {
        matches!(self.kind, ConversionKind::Identity)
    }
}
```

### 5. OpCode (bytecode/opcode.rs)

```rust
/// Bytecode operation codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    // === Constants ===
    /// Push constant from pool (8-bit index)
    Constant = 0,
    /// Push constant from pool (16-bit index)
    ConstantWide,
    /// Push null handle
    PushNull,
    /// Push boolean true
    PushTrue,
    /// Push boolean false
    PushFalse,
    /// Push integer 0
    PushZero,
    /// Push integer 1
    PushOne,

    // === Stack ===
    /// Pop top of stack
    Pop,
    /// Pop N values from stack
    PopN,
    /// Duplicate top of stack
    Dup,

    // === Locals ===
    /// Load local variable (8-bit slot)
    GetLocal,
    /// Store to local variable (8-bit slot)
    SetLocal,
    /// Load local variable (16-bit slot)
    GetLocalWide,
    /// Store to local variable (16-bit slot)
    SetLocalWide,

    // === Globals ===
    /// Load global by hash (from constant pool)
    GetGlobal,
    /// Store to global by hash
    SetGlobal,

    // === Object Fields ===
    /// Load field by index
    GetField,
    /// Store to field by index
    SetField,
    /// Push 'this' reference
    GetThis,

    // === Arithmetic (i32) ===
    AddI32,
    SubI32,
    MulI32,
    DivI32,
    ModI32,
    NegI32,

    // === Arithmetic (i64) ===
    AddI64,
    SubI64,
    MulI64,
    DivI64,
    ModI64,
    NegI64,

    // === Arithmetic (f32) ===
    AddF32,
    SubF32,
    MulF32,
    DivF32,
    NegF32,

    // === Arithmetic (f64) ===
    AddF64,
    SubF64,
    MulF64,
    DivF64,
    NegF64,

    // === Bitwise ===
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
    Ushr,

    // === Comparison (produce bool) ===
    EqI32,
    EqI64,
    EqF32,
    EqF64,
    EqBool,
    EqHandle,  // Reference equality

    LtI32,
    LtI64,
    LtF32,
    LtF64,

    LeI32,
    LeI64,
    LeF32,
    LeF64,

    GtI32,
    GtI64,
    GtF32,
    GtF64,

    GeI32,
    GeI64,
    GeF32,
    GeF64,

    // === Logical ===
    Not,

    // === Control Flow ===
    /// Unconditional jump (16-bit signed offset)
    Jump,
    /// Jump if top of stack is false
    JumpIfFalse,
    /// Jump if top of stack is true
    JumpIfTrue,
    /// Jump backward (for loops)
    Loop,

    // === Calls ===
    /// Call function (hash in constant pool, arg count follows)
    Call,
    /// Call method on object
    CallMethod,
    /// Call virtual method (interface dispatch)
    CallVirtual,
    /// Return from function with value
    Return,
    /// Return from void function
    ReturnVoid,

    // === Object Creation ===
    /// Allocate object and call constructor
    New,
    /// Call factory function
    NewFactory,

    // === Type Conversions ===
    // Integer widening
    I8toI16,
    I8toI32,
    I8toI64,
    I16toI32,
    I16toI64,
    I32toI64,
    U8toU16,
    U8toU32,
    U8toU64,
    U16toU32,
    U16toU64,
    U32toU64,

    // Integer narrowing
    I64toI32,
    I64toI16,
    I64toI8,
    I32toI16,
    I32toI8,
    I16toI8,

    // Float conversions
    I32toF32,
    I32toF64,
    I64toF32,
    I64toF64,
    F32toI32,
    F32toI64,
    F64toI32,
    F64toI64,
    F32toF64,
    F64toF32,

    // Handle conversions
    HandleToConst,
    DerivedToBase,
    ClassToInterface,
    ValueToHandle,

    // === Type Checking ===
    /// Check if handle is instance of type
    InstanceOf,
    /// Explicit cast (may fail at runtime)
    Cast,

    // === Function Pointers ===
    /// Create function pointer
    FuncPtr,
    /// Call through function pointer
    CallFuncPtr,

    // === Init Lists ===
    /// Begin init list of size N
    InitListBegin,
    /// End init list
    InitListEnd,

    // === Handles ===
    /// Increment reference count
    AddRef,
    /// Decrement reference count
    Release,
}
```

### 6. BytecodeChunk (bytecode/chunk.rs)

```rust
use super::{Constant, OpCode};

/// A chunk of compiled bytecode with associated data.
#[derive(Debug, Clone, Default)]
pub struct BytecodeChunk {
    /// The bytecode instructions
    pub code: Vec<u8>,
    /// Constant pool
    pub constants: Vec<Constant>,
    /// Line numbers for debugging (parallel to code)
    pub lines: Vec<u32>,
}

impl BytecodeChunk {
    pub fn new() -> Self {
        Self::default()
    }

    /// Write an opcode
    pub fn write_op(&mut self, op: OpCode, line: u32) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    /// Write a byte operand
    pub fn write_byte(&mut self, byte: u8, line: u32) {
        self.code.push(byte);
        self.lines.push(line);
    }

    /// Write a 16-bit operand
    pub fn write_u16(&mut self, value: u16, line: u32) {
        self.code.push((value >> 8) as u8);
        self.lines.push(line);
        self.code.push(value as u8);
        self.lines.push(line);
    }

    /// Add a constant and return its index
    pub fn add_constant(&mut self, constant: Constant) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }

    /// Get current code offset (for jump patching)
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// Patch a jump instruction at the given offset
    pub fn patch_jump(&mut self, offset: usize) {
        let jump_distance = self.code.len() - offset - 2;
        self.code[offset] = (jump_distance >> 8) as u8;
        self.code[offset + 1] = jump_distance as u8;
    }
}
```

### 7. Constants (bytecode/constant.rs)

```rust
use angelscript_core::TypeHash;

/// Values stored in the constant pool.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Signed integer (i64 to support all int sizes)
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// 32-bit float
    Float32(f32),
    /// 64-bit float
    Float64(f64),
    /// String literal
    String(String),
    /// Type hash (for function calls, type checks, etc.)
    TypeHash(TypeHash),
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expr_info_rvalue() {
        let info = ExprInfo::rvalue(DataType::simple(primitives::INT32));
        assert!(!info.is_lvalue);
        assert!(!info.is_assignable());
    }

    #[test]
    fn expr_info_lvalue() {
        let info = ExprInfo::lvalue(DataType::simple(primitives::INT32));
        assert!(info.is_lvalue);
        assert!(info.is_assignable());
    }

    #[test]
    fn expr_info_const_lvalue() {
        let info = ExprInfo::const_lvalue(DataType::simple(primitives::INT32));
        assert!(info.is_lvalue);
        assert!(!info.is_assignable());
    }

    #[test]
    fn conversion_identity() {
        let conv = Conversion::identity();
        assert!(conv.is_exact());
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, 0);
    }

    #[test]
    fn bytecode_chunk_write() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(0, 1);
        assert_eq!(chunk.code.len(), 2);
        assert_eq!(chunk.lines.len(), 2);
    }
}
```

## Acceptance Criteria

- [ ] `angelscript-compiler` crate builds successfully
- [ ] All core types are defined and documented
- [ ] Error types cover all compilation error cases
- [ ] OpCode enum is complete for basic operations
- [ ] BytecodeChunk can write and patch instructions
- [ ] All unit tests pass
- [ ] `cargo clippy` passes

## Next Phase

Task 32: Compilation Context - wraps TypeRegistry with compilation state
