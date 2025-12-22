//! Bytecode operation codes.
//!
//! This module defines the instruction set for the AngelScript VM.
//! Each opcode is a single byte, with operands following inline.

/// Bytecode operation codes.
///
/// The VM is a stack-based machine. Most operations pop operands
/// from the stack and push results back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OpCode {
    // =========================================================================
    // Constants
    // =========================================================================
    /// Push constant from pool (8-bit index).
    /// Operand: u8 constant index
    Constant = 0,
    /// Push constant from pool (16-bit index).
    /// Operand: u16 constant index (big-endian)
    ConstantWide,
    /// Push null handle.
    PushNull,
    /// Push boolean true.
    PushTrue,
    /// Push boolean false.
    PushFalse,
    /// Push integer 0.
    PushZero,
    /// Push integer 1.
    PushOne,

    // =========================================================================
    // Stack Operations
    // =========================================================================
    /// Pop top of stack.
    Pop,
    /// Pop N values from stack.
    /// Operand: u8 count
    PopN,
    /// Duplicate top of stack.
    Dup,
    /// Copy value at offset from top of stack to top.
    /// Operand: u8 offset (0 = top, 1 = second from top, etc.)
    /// Like Forth's PICK: stack[n] -> top (without removing original)
    Pick,

    // =========================================================================
    // Local Variables
    // =========================================================================
    /// Load local variable (8-bit slot).
    /// Operand: u8 slot index
    GetLocal,
    /// Store to local variable (8-bit slot).
    /// Operand: u8 slot index
    SetLocal,
    /// Load local variable (16-bit slot).
    /// Operand: u16 slot index (big-endian)
    GetLocalWide,
    /// Store to local variable (16-bit slot).
    /// Operand: u16 slot index (big-endian)
    SetLocalWide,

    // =========================================================================
    // Global Variables
    // =========================================================================
    /// Load global by hash (from constant pool).
    /// Operand: u8 or u16 constant index (TypeHash)
    GetGlobal,
    /// Store to global by hash.
    /// Operand: u8 or u16 constant index (TypeHash)
    SetGlobal,

    // =========================================================================
    // Object Fields
    // =========================================================================
    /// Load field by index.
    /// Operand: u16 field index
    GetField,
    /// Store to field by index.
    /// Operand: u16 field index
    SetField,
    /// Push 'this' reference.
    GetThis,

    // =========================================================================
    // Arithmetic (i32)
    // =========================================================================
    /// Add two i32 values.
    AddI32,
    /// Subtract two i32 values.
    SubI32,
    /// Multiply two i32 values.
    MulI32,
    /// Divide two i32 values.
    DivI32,
    /// Modulo of two i32 values.
    ModI32,
    /// Negate i32 value.
    NegI32,
    /// Exponentiation i32 base with u32 exponent.
    PowI32,

    // =========================================================================
    // Arithmetic (i64)
    // =========================================================================
    /// Add two i64 values.
    AddI64,
    /// Subtract two i64 values.
    SubI64,
    /// Multiply two i64 values.
    MulI64,
    /// Divide two i64 values.
    DivI64,
    /// Modulo of two i64 values.
    ModI64,
    /// Negate i64 value.
    NegI64,
    /// Exponentiation i64 base with u32 exponent.
    PowI64,

    // =========================================================================
    // Arithmetic (f32)
    // =========================================================================
    /// Add two f32 values.
    AddF32,
    /// Subtract two f32 values.
    SubF32,
    /// Multiply two f32 values.
    MulF32,
    /// Divide two f32 values.
    DivF32,
    /// Negate f32 value.
    NegF32,
    /// Exponentiation of two f32 values.
    PowF32,

    // =========================================================================
    // Arithmetic (f64)
    // =========================================================================
    /// Add two f64 values.
    AddF64,
    /// Subtract two f64 values.
    SubF64,
    /// Multiply two f64 values.
    MulF64,
    /// Divide two f64 values.
    DivF64,
    /// Negate f64 value.
    NegF64,
    /// Exponentiation of two f64 values.
    PowF64,

    // =========================================================================
    // Bitwise Operations
    // =========================================================================
    /// Bitwise AND.
    BitAnd,
    /// Bitwise OR.
    BitOr,
    /// Bitwise XOR.
    BitXor,
    /// Bitwise NOT.
    BitNot,
    /// Shift left.
    Shl,
    /// Arithmetic shift right (signed).
    Shr,
    /// Logical shift right (unsigned).
    Ushr,

    // =========================================================================
    // Comparisons (produce bool)
    // =========================================================================
    /// Compare i32 equality.
    EqI32,
    /// Compare i64 equality.
    EqI64,
    /// Compare f32 equality.
    EqF32,
    /// Compare f64 equality.
    EqF64,
    /// Compare bool equality.
    EqBool,
    /// Reference equality for handles.
    EqHandle,

    /// i32 less than.
    LtI32,
    /// i64 less than.
    LtI64,
    /// f32 less than.
    LtF32,
    /// f64 less than.
    LtF64,

    /// i32 less than or equal.
    LeI32,
    /// i64 less than or equal.
    LeI64,
    /// f32 less than or equal.
    LeF32,
    /// f64 less than or equal.
    LeF64,

    /// i32 greater than.
    GtI32,
    /// i64 greater than.
    GtI64,
    /// f32 greater than.
    GtF32,
    /// f64 greater than.
    GtF64,

    /// i32 greater than or equal.
    GeI32,
    /// i64 greater than or equal.
    GeI64,
    /// f32 greater than or equal.
    GeF32,
    /// f64 greater than or equal.
    GeF64,

    // =========================================================================
    // Logical Operations
    // =========================================================================
    /// Logical NOT.
    Not,

    // =========================================================================
    // Control Flow
    // =========================================================================
    /// Unconditional jump (16-bit signed offset).
    /// Operand: i16 offset (big-endian)
    Jump,
    /// Jump if top of stack is false.
    /// Operand: i16 offset (big-endian)
    JumpIfFalse,
    /// Jump if top of stack is true.
    /// Operand: i16 offset (big-endian)
    JumpIfTrue,
    /// Jump backward (for loops).
    /// Operand: u16 offset (big-endian)
    Loop,

    // =========================================================================
    // Function Calls
    // =========================================================================
    /// Call function (hash in constant pool).
    /// Operands: u8/u16 constant index (TypeHash), u8 arg count
    Call,
    /// Call method on object (direct dispatch).
    /// Operands: u8/u16 constant index (method hash), u8 arg count
    CallMethod,
    /// Call virtual method using vtable slot (for polymorphic class dispatch).
    /// Stack: [obj, args...] -> [result]
    /// Operands: u16 vtable slot index, u8 arg count
    CallVirtual,
    /// Call interface method using itable (for polymorphic interface dispatch).
    /// Stack: [obj, args...] -> [result]
    /// Operands: u16 constant index (interface TypeHash), u16 slot index, u8 arg count
    CallInterface,
    /// Return from function with value.
    Return,
    /// Return from void function.
    ReturnVoid,

    // =========================================================================
    // Object Creation
    // =========================================================================
    /// Allocate object and call constructor.
    /// Operands: u8/u16 constant index (type hash), u8 arg count
    New,
    /// Call factory function.
    /// Operands: u8/u16 constant index (factory hash), u8 arg count
    NewFactory,

    // =========================================================================
    // Type Conversions - Integer Widening
    // =========================================================================
    /// Convert i8 to i16.
    I8toI16,
    /// Convert i8 to i32.
    I8toI32,
    /// Convert i8 to i64.
    I8toI64,
    /// Convert i16 to i32.
    I16toI32,
    /// Convert i16 to i64.
    I16toI64,
    /// Convert i32 to i64.
    I32toI64,
    /// Convert u8 to u16.
    U8toU16,
    /// Convert u8 to u32.
    U8toU32,
    /// Convert u8 to u64.
    U8toU64,
    /// Convert u16 to u32.
    U16toU32,
    /// Convert u16 to u64.
    U16toU64,
    /// Convert u32 to u64.
    U32toU64,

    // =========================================================================
    // Type Conversions - Integer Narrowing
    // =========================================================================
    /// Convert i64 to i32.
    I64toI32,
    /// Convert i64 to i16.
    I64toI16,
    /// Convert i64 to i8.
    I64toI8,
    /// Convert i32 to i16.
    I32toI16,
    /// Convert i32 to i8.
    I32toI8,
    /// Convert i16 to i8.
    I16toI8,

    // =========================================================================
    // Type Conversions - Float
    // =========================================================================
    /// Convert i32 to f32.
    I32toF32,
    /// Convert i32 to f64.
    I32toF64,
    /// Convert i64 to f32.
    I64toF32,
    /// Convert i64 to f64.
    I64toF64,
    /// Convert f32 to i32.
    F32toI32,
    /// Convert f32 to i64.
    F32toI64,
    /// Convert f64 to i32.
    F64toI32,
    /// Convert f64 to i64.
    F64toI64,
    /// Convert f32 to f64.
    F32toF64,
    /// Convert f64 to f32.
    F64toF32,

    // =========================================================================
    // Type Conversions - Handle/Reference
    // =========================================================================
    /// Convert handle to const handle.
    HandleToConst,
    /// Convert derived class to base class.
    /// Operand: u8/u16 constant index (base type hash)
    DerivedToBase,
    /// Convert class to interface.
    /// Operand: u8/u16 constant index (interface type hash)
    ClassToInterface,
    /// Convert value type to handle.
    ValueToHandle,

    // =========================================================================
    // Type Checking
    // =========================================================================
    /// Check if handle is instance of type.
    /// Operand: u8/u16 constant index (type hash)
    /// Pushes bool result.
    InstanceOf,
    /// Explicit cast (may fail at runtime).
    /// Operand: u8/u16 constant index (target type hash)
    Cast,

    // =========================================================================
    // Function Pointers
    // =========================================================================
    /// Create function pointer.
    /// Operand: u8/u16 constant index (function hash)
    FuncPtr,
    /// Call through function pointer.
    /// Operand: u8 arg count
    CallFuncPtr,

    // =========================================================================
    // Init Lists
    // =========================================================================
    /// Begin init list of size N.
    /// Operand: u16 size
    InitListBegin,
    /// End init list.
    InitListEnd,

    // =========================================================================
    // Increment/Decrement
    // =========================================================================
    /// Pre-increment (++x).
    PreInc,
    /// Pre-decrement (--x).
    PreDec,
    /// Post-increment (x++).
    PostInc,
    /// Post-decrement (x--).
    PostDec,

    // =========================================================================
    // Handle Operations
    // =========================================================================
    /// Take handle of value (@value).
    HandleOf,
    /// Swap top two stack values.
    Swap,

    // =========================================================================
    // Reference Counting
    // =========================================================================
    /// Increment reference count.
    AddRef,
    /// Decrement reference count.
    Release,

    // =========================================================================
    // Exception Handling
    // =========================================================================
    /// Begin try block - pushes exception handler.
    /// Operand: i16 offset to catch block (relative to after the offset)
    TryBegin,
    /// End try block - pops exception handler (try completed without exception).
    TryEnd,
}

impl OpCode {
    /// Convert from u8, returning None for invalid values.
    pub fn from_u8(value: u8) -> Option<Self> {
        // Safety: We check bounds before transmuting
        if value <= OpCode::TryEnd as u8 {
            // SAFETY: OpCode is repr(u8) and we've verified the value is in range
            Some(unsafe { std::mem::transmute::<u8, OpCode>(value) })
        } else {
            None
        }
    }

    /// Get the size of operands for this opcode in bytes.
    ///
    /// This does NOT include the opcode byte itself.
    pub fn operand_size(&self) -> usize {
        match self {
            // No operands (1 byte total)
            OpCode::PushNull
            | OpCode::PushTrue
            | OpCode::PushFalse
            | OpCode::PushZero
            | OpCode::PushOne
            | OpCode::Pop
            | OpCode::Dup
            | OpCode::Swap
            | OpCode::GetThis
            | OpCode::AddI32
            | OpCode::SubI32
            | OpCode::MulI32
            | OpCode::DivI32
            | OpCode::ModI32
            | OpCode::NegI32
            | OpCode::PowI32
            | OpCode::AddI64
            | OpCode::SubI64
            | OpCode::MulI64
            | OpCode::DivI64
            | OpCode::ModI64
            | OpCode::NegI64
            | OpCode::PowI64
            | OpCode::AddF32
            | OpCode::SubF32
            | OpCode::MulF32
            | OpCode::DivF32
            | OpCode::NegF32
            | OpCode::PowF32
            | OpCode::AddF64
            | OpCode::SubF64
            | OpCode::MulF64
            | OpCode::DivF64
            | OpCode::NegF64
            | OpCode::PowF64
            | OpCode::BitAnd
            | OpCode::BitOr
            | OpCode::BitXor
            | OpCode::BitNot
            | OpCode::Shl
            | OpCode::Shr
            | OpCode::Ushr
            | OpCode::EqI32
            | OpCode::EqI64
            | OpCode::EqF32
            | OpCode::EqF64
            | OpCode::EqBool
            | OpCode::EqHandle
            | OpCode::LtI32
            | OpCode::LtI64
            | OpCode::LtF32
            | OpCode::LtF64
            | OpCode::LeI32
            | OpCode::LeI64
            | OpCode::LeF32
            | OpCode::LeF64
            | OpCode::GtI32
            | OpCode::GtI64
            | OpCode::GtF32
            | OpCode::GtF64
            | OpCode::GeI32
            | OpCode::GeI64
            | OpCode::GeF32
            | OpCode::GeF64
            | OpCode::Not
            | OpCode::Return
            | OpCode::ReturnVoid
            | OpCode::I8toI16
            | OpCode::I8toI32
            | OpCode::I8toI64
            | OpCode::I16toI32
            | OpCode::I16toI64
            | OpCode::I32toI64
            | OpCode::U8toU16
            | OpCode::U8toU32
            | OpCode::U8toU64
            | OpCode::U16toU32
            | OpCode::U16toU64
            | OpCode::U32toU64
            | OpCode::I64toI32
            | OpCode::I64toI16
            | OpCode::I64toI8
            | OpCode::I32toI16
            | OpCode::I32toI8
            | OpCode::I16toI8
            | OpCode::I32toF32
            | OpCode::I32toF64
            | OpCode::I64toF32
            | OpCode::I64toF64
            | OpCode::F32toI32
            | OpCode::F32toI64
            | OpCode::F64toI32
            | OpCode::F64toI64
            | OpCode::F32toF64
            | OpCode::F64toF32
            | OpCode::HandleToConst
            | OpCode::ValueToHandle
            | OpCode::PreInc
            | OpCode::PreDec
            | OpCode::PostInc
            | OpCode::PostDec
            | OpCode::HandleOf
            | OpCode::AddRef
            | OpCode::Release
            | OpCode::TryEnd
            | OpCode::InitListEnd => 0,

            // 1-byte operand
            OpCode::Constant // u8 constant index
            | OpCode::PopN   // u8 count
            | OpCode::Pick   // u8 offset
            | OpCode::GetLocal  // u8 slot
            | OpCode::SetLocal  // u8 slot
            | OpCode::CallFuncPtr => 1, // u8 arg count

            // 2-byte operand
            OpCode::ConstantWide // u16 constant index
            | OpCode::GetLocalWide  // u16 slot
            | OpCode::SetLocalWide  // u16 slot
            | OpCode::GetGlobal     // u16 constant index (TypeHash)
            | OpCode::SetGlobal     // u16 constant index (TypeHash)
            | OpCode::GetField      // u16 field index
            | OpCode::SetField      // u16 field index
            | OpCode::Jump          // i16 offset
            | OpCode::JumpIfFalse   // i16 offset
            | OpCode::JumpIfTrue    // i16 offset
            | OpCode::Loop          // u16 offset
            | OpCode::DerivedToBase     // u16 constant index
            | OpCode::ClassToInterface  // u16 constant index
            | OpCode::InstanceOf        // u16 constant index
            | OpCode::Cast              // u16 constant index
            | OpCode::FuncPtr           // u16 constant index
            | OpCode::InitListBegin     // u16 size
            | OpCode::TryBegin => 2, // i16 offset

            // 3-byte operand (u16 + u8)
            OpCode::Call        // u16 constant index + u8 arg count
            | OpCode::CallMethod    // u16 constant index + u8 arg count
            | OpCode::CallVirtual   // u16 vtable slot + u8 arg count
            | OpCode::New           // u16 constant index + u8 arg count
            | OpCode::NewFactory => 3, // u16 constant index + u8 arg count

            // 5-byte operand (u16 + u16 + u8)
            OpCode::CallInterface => 5, // u16 iface hash constant + u16 slot + u8 arg count
        }
    }

    /// Get the name of this opcode for debugging.
    pub fn name(&self) -> &'static str {
        match self {
            OpCode::Constant => "CONSTANT",
            OpCode::ConstantWide => "CONSTANT_WIDE",
            OpCode::PushNull => "PUSH_NULL",
            OpCode::PushTrue => "PUSH_TRUE",
            OpCode::PushFalse => "PUSH_FALSE",
            OpCode::PushZero => "PUSH_ZERO",
            OpCode::PushOne => "PUSH_ONE",
            OpCode::Pop => "POP",
            OpCode::PopN => "POP_N",
            OpCode::Dup => "DUP",
            OpCode::Pick => "PICK",
            OpCode::GetLocal => "GET_LOCAL",
            OpCode::SetLocal => "SET_LOCAL",
            OpCode::GetLocalWide => "GET_LOCAL_WIDE",
            OpCode::SetLocalWide => "SET_LOCAL_WIDE",
            OpCode::GetGlobal => "GET_GLOBAL",
            OpCode::SetGlobal => "SET_GLOBAL",
            OpCode::GetField => "GET_FIELD",
            OpCode::SetField => "SET_FIELD",
            OpCode::GetThis => "GET_THIS",
            OpCode::AddI32 => "ADD_I32",
            OpCode::SubI32 => "SUB_I32",
            OpCode::MulI32 => "MUL_I32",
            OpCode::DivI32 => "DIV_I32",
            OpCode::ModI32 => "MOD_I32",
            OpCode::NegI32 => "NEG_I32",
            OpCode::PowI32 => "POW_I32",
            OpCode::AddI64 => "ADD_I64",
            OpCode::SubI64 => "SUB_I64",
            OpCode::MulI64 => "MUL_I64",
            OpCode::DivI64 => "DIV_I64",
            OpCode::ModI64 => "MOD_I64",
            OpCode::NegI64 => "NEG_I64",
            OpCode::PowI64 => "POW_I64",
            OpCode::AddF32 => "ADD_F32",
            OpCode::SubF32 => "SUB_F32",
            OpCode::MulF32 => "MUL_F32",
            OpCode::DivF32 => "DIV_F32",
            OpCode::NegF32 => "NEG_F32",
            OpCode::PowF32 => "POW_F32",
            OpCode::AddF64 => "ADD_F64",
            OpCode::SubF64 => "SUB_F64",
            OpCode::MulF64 => "MUL_F64",
            OpCode::DivF64 => "DIV_F64",
            OpCode::NegF64 => "NEG_F64",
            OpCode::PowF64 => "POW_F64",
            OpCode::BitAnd => "BIT_AND",
            OpCode::BitOr => "BIT_OR",
            OpCode::BitXor => "BIT_XOR",
            OpCode::BitNot => "BIT_NOT",
            OpCode::Shl => "SHL",
            OpCode::Shr => "SHR",
            OpCode::Ushr => "USHR",
            OpCode::EqI32 => "EQ_I32",
            OpCode::EqI64 => "EQ_I64",
            OpCode::EqF32 => "EQ_F32",
            OpCode::EqF64 => "EQ_F64",
            OpCode::EqBool => "EQ_BOOL",
            OpCode::EqHandle => "EQ_HANDLE",
            OpCode::LtI32 => "LT_I32",
            OpCode::LtI64 => "LT_I64",
            OpCode::LtF32 => "LT_F32",
            OpCode::LtF64 => "LT_F64",
            OpCode::LeI32 => "LE_I32",
            OpCode::LeI64 => "LE_I64",
            OpCode::LeF32 => "LE_F32",
            OpCode::LeF64 => "LE_F64",
            OpCode::GtI32 => "GT_I32",
            OpCode::GtI64 => "GT_I64",
            OpCode::GtF32 => "GT_F32",
            OpCode::GtF64 => "GT_F64",
            OpCode::GeI32 => "GE_I32",
            OpCode::GeI64 => "GE_I64",
            OpCode::GeF32 => "GE_F32",
            OpCode::GeF64 => "GE_F64",
            OpCode::Not => "NOT",
            OpCode::Jump => "JUMP",
            OpCode::JumpIfFalse => "JUMP_IF_FALSE",
            OpCode::JumpIfTrue => "JUMP_IF_TRUE",
            OpCode::Loop => "LOOP",
            OpCode::Call => "CALL",
            OpCode::CallMethod => "CALL_METHOD",
            OpCode::CallVirtual => "CALL_VIRTUAL",
            OpCode::CallInterface => "CALL_INTERFACE",
            OpCode::Return => "RETURN",
            OpCode::ReturnVoid => "RETURN_VOID",
            OpCode::New => "NEW",
            OpCode::NewFactory => "NEW_FACTORY",
            OpCode::I8toI16 => "I8_TO_I16",
            OpCode::I8toI32 => "I8_TO_I32",
            OpCode::I8toI64 => "I8_TO_I64",
            OpCode::I16toI32 => "I16_TO_I32",
            OpCode::I16toI64 => "I16_TO_I64",
            OpCode::I32toI64 => "I32_TO_I64",
            OpCode::U8toU16 => "U8_TO_U16",
            OpCode::U8toU32 => "U8_TO_U32",
            OpCode::U8toU64 => "U8_TO_U64",
            OpCode::U16toU32 => "U16_TO_U32",
            OpCode::U16toU64 => "U16_TO_U64",
            OpCode::U32toU64 => "U32_TO_U64",
            OpCode::I64toI32 => "I64_TO_I32",
            OpCode::I64toI16 => "I64_TO_I16",
            OpCode::I64toI8 => "I64_TO_I8",
            OpCode::I32toI16 => "I32_TO_I16",
            OpCode::I32toI8 => "I32_TO_I8",
            OpCode::I16toI8 => "I16_TO_I8",
            OpCode::I32toF32 => "I32_TO_F32",
            OpCode::I32toF64 => "I32_TO_F64",
            OpCode::I64toF32 => "I64_TO_F32",
            OpCode::I64toF64 => "I64_TO_F64",
            OpCode::F32toI32 => "F32_TO_I32",
            OpCode::F32toI64 => "F32_TO_I64",
            OpCode::F64toI32 => "F64_TO_I32",
            OpCode::F64toI64 => "F64_TO_I64",
            OpCode::F32toF64 => "F32_TO_F64",
            OpCode::F64toF32 => "F64_TO_F32",
            OpCode::HandleToConst => "HANDLE_TO_CONST",
            OpCode::DerivedToBase => "DERIVED_TO_BASE",
            OpCode::ClassToInterface => "CLASS_TO_INTERFACE",
            OpCode::ValueToHandle => "VALUE_TO_HANDLE",
            OpCode::InstanceOf => "INSTANCE_OF",
            OpCode::Cast => "CAST",
            OpCode::FuncPtr => "FUNC_PTR",
            OpCode::CallFuncPtr => "CALL_FUNC_PTR",
            OpCode::InitListBegin => "INIT_LIST_BEGIN",
            OpCode::InitListEnd => "INIT_LIST_END",
            OpCode::PreInc => "PRE_INC",
            OpCode::PreDec => "PRE_DEC",
            OpCode::PostInc => "POST_INC",
            OpCode::PostDec => "POST_DEC",
            OpCode::HandleOf => "HANDLE_OF",
            OpCode::Swap => "SWAP",
            OpCode::AddRef => "ADD_REF",
            OpCode::Release => "RELEASE",
            OpCode::TryBegin => "TRY_BEGIN",
            OpCode::TryEnd => "TRY_END",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_repr() {
        assert_eq!(OpCode::Constant as u8, 0);
        assert_eq!(OpCode::ConstantWide as u8, 1);
    }

    #[test]
    fn opcode_from_u8() {
        assert_eq!(OpCode::from_u8(0), Some(OpCode::Constant));
        assert_eq!(OpCode::from_u8(1), Some(OpCode::ConstantWide));
        assert_eq!(OpCode::from_u8(255), None);
    }

    #[test]
    fn opcode_name() {
        assert_eq!(OpCode::Constant.name(), "CONSTANT");
        assert_eq!(OpCode::AddI32.name(), "ADD_I32");
        assert_eq!(OpCode::JumpIfFalse.name(), "JUMP_IF_FALSE");
    }

    #[test]
    fn exception_opcodes() {
        // Verify TryBegin and TryEnd are valid opcodes
        assert_eq!(OpCode::TryBegin.name(), "TRY_BEGIN");
        assert_eq!(OpCode::TryEnd.name(), "TRY_END");

        // Verify they can be converted from u8
        let try_begin_val = OpCode::TryBegin as u8;
        let try_end_val = OpCode::TryEnd as u8;
        assert_eq!(OpCode::from_u8(try_begin_val), Some(OpCode::TryBegin));
        assert_eq!(OpCode::from_u8(try_end_val), Some(OpCode::TryEnd));

        // TryEnd should be the last opcode
        assert_eq!(OpCode::from_u8(try_end_val + 1), None);
    }

    #[test]
    fn operand_sizes() {
        // No operands
        assert_eq!(OpCode::Pop.operand_size(), 0);
        assert_eq!(OpCode::AddI32.operand_size(), 0);
        assert_eq!(OpCode::Return.operand_size(), 0);

        // 1-byte operand
        assert_eq!(OpCode::Constant.operand_size(), 1);
        assert_eq!(OpCode::GetLocal.operand_size(), 1);
        assert_eq!(OpCode::SetLocal.operand_size(), 1);

        // 2-byte operand
        assert_eq!(OpCode::ConstantWide.operand_size(), 2);
        assert_eq!(OpCode::Jump.operand_size(), 2);
        assert_eq!(OpCode::GetField.operand_size(), 2);

        // 3-byte operand
        assert_eq!(OpCode::Call.operand_size(), 3);
        assert_eq!(OpCode::CallMethod.operand_size(), 3);
        assert_eq!(OpCode::New.operand_size(), 3);
    }
}
