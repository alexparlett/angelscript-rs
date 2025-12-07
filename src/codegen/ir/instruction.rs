//! Bytecode instruction set for AngelScript.
//!
//! This module defines the instruction set for the AngelScript bytecode.
//! The bytecode is a simple stack-based instruction set.

use crate::types::TypeHash;

/// A bytecode instruction.
///
/// This is a simplified bytecode representation for the semantic analysis phase.
/// The actual VM bytecode may be different.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Stack operations
    /// Push a constant integer onto the stack
    PushInt(i64),
    /// Push a constant float onto the stack
    PushFloat(f32),
    /// Push a constant double onto the stack
    PushDouble(f64),
    /// Push a boolean constant onto the stack
    PushBool(bool),
    /// Push null onto the stack
    PushNull,
    /// Push a string constant onto the stack
    PushString(u32), // Index into string constant table

    // Local variable operations
    /// Load a local variable onto the stack
    LoadLocal(u32), // Stack offset
    /// Store the top of the stack into a local variable
    StoreLocal(u32), // Stack offset

    // Global variable operations
    /// Load a global variable onto the stack
    LoadGlobal(u32), // Global variable ID
    /// Store the top of the stack into a global variable
    StoreGlobal(u32), // Global variable ID

    // Arithmetic operations
    /// Add two values (pops 2, pushes 1)
    Add,
    /// Subtract two values (pops 2, pushes 1)
    Sub,
    /// Multiply two values (pops 2, pushes 1)
    Mul,
    /// Divide two values (pops 2, pushes 1)
    Div,
    /// Modulo operation (pops 2, pushes 1)
    Mod,
    /// Power operation (pops 2, pushes 1)
    Pow,

    // Bitwise operations
    /// Bitwise AND (pops 2, pushes 1)
    BitAnd,
    /// Bitwise OR (pops 2, pushes 1)
    BitOr,
    /// Bitwise XOR (pops 2, pushes 1)
    BitXor,
    /// Bitwise left shift (pops 2, pushes 1)
    ShiftLeft,
    /// Bitwise right shift (pops 2, pushes 1)
    ShiftRight,
    /// Bitwise unsigned right shift (pops 2, pushes 1)
    ShiftRightUnsigned,

    // Logical operations
    /// Logical AND (pops 2, pushes 1)
    LogicalAnd,
    /// Logical OR (pops 2, pushes 1)
    LogicalOr,
    /// Logical XOR (pops 2, pushes 1)
    LogicalXor,

    // Comparison operations
    /// Equal comparison (pops 2, pushes 1 bool)
    Equal,
    /// Not equal comparison (pops 2, pushes 1 bool)
    NotEqual,
    /// Less than comparison (pops 2, pushes 1 bool)
    LessThan,
    /// Less than or equal comparison (pops 2, pushes 1 bool)
    LessEqual,
    /// Greater than comparison (pops 2, pushes 1 bool)
    GreaterThan,
    /// Greater than or equal comparison (pops 2, pushes 1 bool)
    GreaterEqual,

    // Unary operations
    /// Negate a value (pops 1, pushes 1)
    Negate,
    /// Logical NOT (pops 1, pushes 1)
    Not,
    /// Bitwise NOT (pops 1, pushes 1)
    BitNot,
    /// Pre-increment (pops 1, pushes 1)
    PreIncrement,
    /// Pre-decrement (pops 1, pushes 1)
    PreDecrement,
    /// Post-increment (pops 1, pushes 1)
    PostIncrement,
    /// Post-decrement (pops 1, pushes 1)
    PostDecrement,

    // Control flow
    /// Unconditional jump to offset
    Jump(i32), // Offset (can be negative)
    /// Jump if top of stack is true (pops 1)
    JumpIfTrue(i32),
    /// Jump if top of stack is false (pops 1)
    JumpIfFalse(i32),

    // Function calls
    /// Call a function (pops args, pushes return value)
    /// The number of args is determined by looking up the function definition
    Call(u64), // TypeHash.0

    /// Call a method (pops object + args, pushes return value)
    /// The number of args is determined by looking up the method definition
    CallMethod(u64), // TypeHash.0
    /// Call an interface method (pops object + args, pushes return value)
    /// First u64 is the interface TypeHash, second u32 is the method index in the interface
    CallInterfaceMethod(u64, u32), // (InterfaceTypeId, MethodIndex)
    /// Return from function (pops return value if any)
    Return,
    /// Return void (no value)
    ReturnVoid,

    // Object operations
    /// Load the implicit 'this' object reference in a method/constructor
    /// Stack: [...] → [... this]
    LoadThis,
    /// Load a field from an object (pops object, pushes field value)
    LoadField(u32), // Field index
    /// Store a value into an object field (pops value and object)
    StoreField(u32), // Field index
    /// Store a handle value (pops value and target address, stores reference)
    /// Used for @handle = value; syntax
    StoreHandle,
    /// Convert a value type to a handle (e.g., Node -> Node@)
    /// Used when initializing handles with value type expressions
    ValueToHandle,

    // Type operations
    /// Cast to a type (pops 1, pushes 1)
    Cast(TypeHash),
    /// Check if handle on stack is instance of type (including subclasses/interfaces)
    /// Stack: [handle] → [bool]
    IsInstanceOf(TypeHash),

    // Type conversion operations - Primitive conversions
    // Integer to Float conversions
    ConvertI8F32,
    ConvertI16F32,
    ConvertI32F32,
    ConvertI64F32,
    ConvertI8F64,
    ConvertI16F64,
    ConvertI32F64,
    ConvertI64F64,

    // Unsigned to Float conversions
    ConvertU8F32,
    ConvertU16F32,
    ConvertU32F32,
    ConvertU64F32,
    ConvertU8F64,
    ConvertU16F64,
    ConvertU32F64,
    ConvertU64F64,

    // Float to Integer conversions (truncate)
    ConvertF32I8,
    ConvertF32I16,
    ConvertF32I32,
    ConvertF32I64,
    ConvertF32U8,
    ConvertF32U16,
    ConvertF32U32,
    ConvertF32U64,
    ConvertF64I8,
    ConvertF64I16,
    ConvertF64I32,
    ConvertF64I64,
    ConvertF64U8,
    ConvertF64U16,
    ConvertF64U32,
    ConvertF64U64,

    // Float to Float conversions
    ConvertF32F64,
    ConvertF64F32,

    // Integer widening (signed)
    ConvertI8I16,
    ConvertI8I32,
    ConvertI8I64,
    ConvertI16I32,
    ConvertI16I64,
    ConvertI32I64,

    // Integer narrowing (signed)
    ConvertI64I32,
    ConvertI64I16,
    ConvertI64I8,
    ConvertI32I16,
    ConvertI32I8,
    ConvertI16I8,

    // Unsigned widening
    ConvertU8U16,
    ConvertU8U32,
    ConvertU8U64,
    ConvertU16U32,
    ConvertU16U64,
    ConvertU32U64,

    // Unsigned narrowing
    ConvertU64U32,
    ConvertU64U16,
    ConvertU64U8,
    ConvertU32U16,
    ConvertU32U8,
    ConvertU16U8,

    // Signed/Unsigned conversions (same size, reinterpret)
    ConvertI8U8,
    ConvertI16U16,
    ConvertI32U32,
    ConvertI64U64,
    ConvertU8I8,
    ConvertU16I16,
    ConvertU32I32,
    ConvertU64I64,

    // Handle conversions
    /// Convert handle to const handle (T@ → const T@)
    CastHandleToConst,
    /// Cast derived class handle to base class handle (Derived@ → Base@)
    CastHandleDerivedToBase,
    /// Cast class handle to interface handle (Class@ → Interface@)
    CastHandleToInterface,
    /// Explicit handle cast via opCast() - may fail at runtime
    CastHandleExplicit,

    // Constructor and method calls for user-defined conversions
    /// Call a constructor to create a new object
    ///
    /// VM responsibilities:
    /// 1. Allocate object of the specified type
    /// 2. Initialize all fields to defaults (in declaration order)
    /// 3. Call base class constructor if needed
    /// 4. Execute constructor body bytecode (func_id)
    /// 5. Push object handle onto stack
    ///
    /// Fields: (type_id, func_id)
    /// - type_id: TypeHash of the class being constructed
    /// - func_id: TypeHash of the constructor to call
    CallConstructor { type_id: u64, func_id: u64 },

    /// Call a factory function for a reference type.
    /// Similar to CallConstructor but for reference types which use factory
    /// functions instead of constructors.
    ///
    /// Execution:
    /// 1. Pop arguments from stack (based on factory signature)
    /// 2. Call factory function
    /// 3. Factory allocates and initializes the object
    /// 4. Push object handle onto stack
    ///
    /// Fields: (type_id, func_id)
    /// - type_id: TypeHash of the class being created
    /// - func_id: TypeHash of the factory to call
    CallFactory { type_id: u64, func_id: u64 },

    // Stack management
    /// Pop the top value from the stack
    Pop,
    /// Duplicate the top value on the stack
    Dup,
    /// Swap the top two values on the stack
    /// Stack before: [... a b] (top is b)
    /// Stack after: [... b a] (top is a)
    Swap,

    // Exception handling
    /// Start of try block
    TryStart,
    /// End of try block
    TryEnd,
    /// Start of catch block
    CatchStart,
    /// End of catch block
    CatchEnd,

    // Special
    /// No operation
    Nop,

    // Lambda/Funcdef support
    /// Push a function pointer onto the stack
    /// Used for lambda expressions and function references
    /// The function pointer is a handle (reference-counted) to the function
    /// Stack: [...] → [... funcdef_handle]
    FuncPtr(u64), // TypeHash.0 - creates a handle to this function

    /// Call through a function pointer (funcdef)
    /// The funcdef handle is already on the stack (loaded from a variable)
    /// Pops: funcdef handle (extracts TypeHash from it), then N arguments
    /// Pushes: return value
    /// Stack: [funcdef_handle arg1 arg2 ...] → [return_value]
    CallPtr,

    // Initialization list operations
    // These support complex initializers like: array<int> a = {1, 2, 3};
    // and dictionary: dictionary d = {{"key1", val1}, {"key2", val2}};
    //
    // The approach: Build a buffer containing the init data, then pass it to
    // a list constructor/factory. This supports heterogeneous and nested lists.

    /// Allocate a buffer for initialization list data
    /// - buffer_var: Local variable slot to store buffer pointer
    /// - size: Size in bytes to allocate
    /// Stack: [...] → [...]
    /// Side effect: Stores buffer pointer in local variable
    AllocListBuffer { buffer_var: u32, size: u32 },

    /// Set the element count in the list buffer
    /// - buffer_var: Local variable containing buffer pointer
    /// - offset: Byte offset in buffer where count goes
    /// - count: Number of elements
    /// Stack: [...] → [...]
    SetListSize { buffer_var: u32, offset: u32, count: u32 },

    /// Push pointer to a specific position in the list buffer onto stack
    /// Used to write element values into the buffer
    /// - buffer_var: Local variable containing buffer pointer
    /// - offset: Byte offset to element position
    /// Stack: [...] → [... buffer_ptr+offset]
    PushListElement { buffer_var: u32, offset: u32 },

    /// Set the type ID at a position in the list buffer
    /// Used for dictionary's `?` pattern (heterogeneous values)
    /// - buffer_var: Local variable containing buffer pointer
    /// - offset: Byte offset where type ID goes
    /// - type_id: TypeHash of the element
    /// Stack: [...] → [...]
    SetListType { buffer_var: u32, offset: u32, type_id: u64 },

    /// Free/release an initialization list buffer
    /// Called after the list constructor has consumed the buffer
    /// - buffer_var: Local variable containing buffer pointer
    /// - pattern_type_id: TypeHash of the list pattern type (for proper cleanup)
    /// Stack: [...] → [...]
    FreeListBuffer { buffer_var: u32, pattern_type_id: u64 },
}
