/// Bytecode instruction opcodes for the AngelScript VM.
///
/// Ported from the C++ `asEBCInstr` enum. Each opcode specifies:
/// - What operation to perform
/// - Implicit stack effects (push/pop)
/// - Argument format (encoded in the instruction word)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum OpCode {
    // ── Stack operations ──────────────────────────────────────────────
    /// Pop a pointer from the stack.
    PopPtr = 0,
    /// Push a global pointer onto the stack.
    PshGPtr,
    /// Push a 32-bit constant onto the stack.
    PshC4,
    /// Push a 32-bit variable onto the stack.
    PshV4,
    /// Push the stack frame pointer + offset onto the stack.
    PSF,
    /// Swap the top two pointers on the stack.
    SwapPtr,
    /// Push null onto the stack.
    PshNull,
    /// Push a 64-bit constant onto the stack.
    PshC8,
    /// Push a 64-bit variable onto the stack.
    PshV8,
    /// Push a pointer-sized variable onto the stack.
    PshVPtr,
    /// Push register pointer onto the stack.
    PshRPtr,
    /// Read pointer at stack top and push dereferenced pointer.
    RDSPtr,
    /// Pop register pointer from stack.
    PopRPtr,
    /// Push a 32-bit global variable onto the stack.
    PshG4,
    /// Clear a pointer-sized variable (set to null).
    ClrVPtr,

    // ── 32-bit value operations ───────────────────────────────────────
    /// Set a 1-byte variable.
    SetV1,
    /// Set a 2-byte variable.
    SetV2,
    /// Set a 4-byte variable.
    SetV4,
    /// Set an 8-byte variable.
    SetV8,
    /// Set a 4-byte global.
    SetG4,

    // ── Copy operations ───────────────────────────────────────────────
    /// Copy 4-byte variable to variable.
    CpyVtoV4,
    /// Copy 8-byte variable to variable.
    CpyVtoV8,
    /// Copy 4-byte variable to register.
    CpyVtoR4,
    /// Copy 8-byte variable to register.
    CpyVtoR8,
    /// Copy 4-byte variable to global.
    CpyVtoG4,
    /// Copy 4-byte register to variable.
    CpyRtoV4,
    /// Copy 8-byte register to variable.
    CpyRtoV8,
    /// Copy 4-byte global to variable.
    CpyGtoV4,

    // ── Write/Read through register pointer ──────────────────────────
    /// Write 1-byte variable through register pointer.
    WRTV1,
    /// Write 2-byte variable through register pointer.
    WRTV2,
    /// Write 4-byte variable through register pointer.
    WRTV4,
    /// Write 8-byte variable through register pointer.
    WRTV8,
    /// Read 1-byte through register pointer into variable.
    RDR1,
    /// Read 2-byte through register pointer into variable.
    RDR2,
    /// Read 4-byte through register pointer into variable.
    RDR4,
    /// Read 8-byte through register pointer into variable.
    RDR8,

    // ── Load addresses ────────────────────────────────────────────────
    /// Load global variable address into register.
    LDG,
    /// Load local variable address into register.
    LDV,
    /// Push global address onto stack.
    PGA,
    /// Load variable info (type).
    VAR,

    // ── Integer arithmetic ────────────────────────────────────────────
    /// Add two 32-bit integers.
    ADDi,
    /// Subtract two 32-bit integers.
    SUBi,
    /// Multiply two 32-bit integers.
    MULi,
    /// Divide two signed 32-bit integers.
    DIVi,
    /// Modulo two signed 32-bit integers.
    MODi,
    /// Divide two unsigned 32-bit integers.
    DIVu,
    /// Modulo two unsigned 32-bit integers.
    MODu,
    /// Add immediate 32-bit integer.
    ADDIi,
    /// Subtract immediate 32-bit integer.
    SUBIi,
    /// Multiply immediate 32-bit integer.
    MULIi,

    // ── Float arithmetic ──────────────────────────────────────────────
    /// Add two 32-bit floats.
    ADDf,
    /// Subtract two 32-bit floats.
    SUBf,
    /// Multiply two 32-bit floats.
    MULf,
    /// Divide two 32-bit floats.
    DIVf,
    /// Modulo two 32-bit floats.
    MODf,
    /// Add immediate 32-bit float.
    ADDIf,
    /// Subtract immediate 32-bit float.
    SUBIf,
    /// Multiply immediate 32-bit float.
    MULIf,

    // ── Double arithmetic ─────────────────────────────────────────────
    /// Add two 64-bit doubles.
    ADDd,
    /// Subtract two 64-bit doubles.
    SUBd,
    /// Multiply two 64-bit doubles.
    MULd,
    /// Divide two 64-bit doubles.
    DIVd,
    /// Modulo two 64-bit doubles.
    MODd,

    // ── 64-bit integer arithmetic ─────────────────────────────────────
    /// Add two 64-bit integers.
    ADDi64,
    /// Subtract two 64-bit integers.
    SUBi64,
    /// Multiply two 64-bit integers.
    MULi64,
    /// Divide two signed 64-bit integers.
    DIVi64,
    /// Modulo two signed 64-bit integers.
    MODi64,
    /// Divide two unsigned 64-bit integers.
    DIVu64,
    /// Modulo two unsigned 64-bit integers.
    MODu64,

    // ── Negate ────────────────────────────────────────────────────────
    /// Negate 32-bit integer.
    NEGi,
    /// Negate 32-bit float.
    NEGf,
    /// Negate 64-bit double.
    NEGd,
    /// Negate 64-bit integer.
    NEGi64,

    // ── Increment/Decrement ──────────────────────────────────────────
    /// Increment 8-bit integer at register.
    INCi8,
    /// Increment 16-bit integer at register.
    INCi16,
    /// Increment 32-bit integer at register.
    INCi,
    /// Increment 32-bit float at register.
    INCf,
    /// Increment 64-bit double at register.
    INCd,
    /// Increment 64-bit integer at register.
    INCi64,
    /// Decrement 8-bit integer at register.
    DECi8,
    /// Decrement 16-bit integer at register.
    DECi16,
    /// Decrement 32-bit integer at register.
    DECi,
    /// Decrement 32-bit float at register.
    DECf,
    /// Decrement 64-bit double at register.
    DECd,
    /// Decrement 64-bit integer at register.
    DECi64,
    /// Increment 32-bit integer variable.
    IncVi,
    /// Decrement 32-bit integer variable.
    DecVi,

    // ── Bitwise operations ────────────────────────────────────────────
    /// Bitwise NOT 32-bit.
    BNOT,
    /// Bitwise AND 32-bit.
    BAND,
    /// Bitwise OR 32-bit.
    BOR,
    /// Bitwise XOR 32-bit.
    BXOR,
    /// Bit shift left 32-bit.
    BSLL,
    /// Bit shift right logical 32-bit.
    BSRL,
    /// Bit shift right arithmetic 32-bit.
    BSRA,

    // ── 64-bit bitwise operations ─────────────────────────────────────
    /// Bitwise NOT 64-bit.
    BNOT64,
    /// Bitwise AND 64-bit.
    BAND64,
    /// Bitwise OR 64-bit.
    BOR64,
    /// Bitwise XOR 64-bit.
    BXOR64,
    /// Bit shift left 64-bit.
    BSLL64,
    /// Bit shift right logical 64-bit.
    BSRL64,
    /// Bit shift right arithmetic 64-bit.
    BSRA64,

    // ── Comparisons ──────────────────────────────────────────────────
    /// Compare two signed 32-bit integers.
    CMPi,
    /// Compare two unsigned 32-bit integers.
    CMPu,
    /// Compare two 32-bit floats.
    CMPf,
    /// Compare two 64-bit doubles.
    CMPd,
    /// Compare two signed 64-bit integers.
    CMPi64,
    /// Compare two unsigned 64-bit integers.
    CMPu64,
    /// Compare two pointers.
    CmpPtr,
    /// Compare 32-bit integer with immediate.
    CMPIi,
    /// Compare 32-bit float with immediate.
    CMPIf,
    /// Compare unsigned 32-bit with immediate.
    CMPIu,

    // ── Logic / conditional ──────────────────────────────────────────
    /// Boolean NOT.
    NOT,
    /// Test zero (set register to 1 if value == 0).
    TZ,
    /// Test not zero.
    TNZ,
    /// Test negative (sign bit set).
    TS,
    /// Test not negative.
    TNS,
    /// Test positive (> 0).
    TP,
    /// Test not positive.
    TNP,

    // ── Control flow ──────────────────────────────────────────────────
    /// Call a script function.
    CALL,
    /// Return from function.
    RET,
    /// Unconditional jump.
    JMP,
    /// Jump if zero (false).
    JZ,
    /// Jump if not zero (true).
    JNZ,
    /// Jump if negative.
    JS,
    /// Jump if not negative.
    JNS,
    /// Jump if positive.
    JP,
    /// Jump if not positive.
    JNP,
    /// Jump through table (switch).
    JMPP,
    /// Jump if low word is zero.
    JLowZ,
    /// Jump if low word is not zero.
    JLowNZ,
    /// Call through function pointer.
    CallPtr,
    /// Call system (native) function.
    CALLSYS,
    /// Call bound (imported) function.
    CALLBND,
    /// Call interface method.
    CALLINTF,

    // ── Type conversions ─────────────────────────────────────────────
    /// int to float.
    iTOf,
    /// int to double.
    iTOd,
    /// float to int.
    fTOi,
    /// float to unsigned int.
    fTOu,
    /// float to double.
    fTOd,
    /// double to int.
    dTOi,
    /// double to unsigned int.
    dTOu,
    /// double to float.
    dTOf,
    /// int64 to int.
    i64TOi,
    /// int64 to float.
    i64TOf,
    /// int64 to double.
    i64TOd,
    /// float to int64.
    fTOi64,
    /// double to int64.
    dTOi64,
    /// unsigned to int64.
    uTOi64,
    /// int to int64 (sign extend).
    iTOi64,
    /// unsigned to float.
    uTOf,
    /// unsigned to double.
    uTOd,
    /// float to uint64.
    fTOu64,
    /// double to uint64.
    dTOu64,
    /// uint64 to float.
    u64TOf,
    /// uint64 to double.
    u64TOd,
    /// signed byte to int (sign extend).
    sbTOi,
    /// signed word to int (sign extend).
    swTOi,
    /// unsigned byte to int (zero extend).
    ubTOi,
    /// unsigned word to int (zero extend).
    uwTOi,
    /// int to byte (truncate).
    iTOb,
    /// int to word (truncate).
    iTOw,
    /// Runtime type cast.
    Cast,
    /// Clear high bytes of register.
    ClrHi,

    // ── Object operations ────────────────────────────────────────────
    /// Load object from variable to register.
    LOADOBJ,
    /// Store object from register to variable.
    STOREOBJ,
    /// Get object from stack.
    GETOBJ,
    /// Get object reference from stack.
    GETOBJREF,
    /// Get reference from stack.
    GETREF,
    /// Copy reference.
    REFCPY,
    /// Copy reference to variable.
    RefCpyV,
    /// Check reference not null.
    CHKREF,
    /// Check reference on stack not null.
    ChkRefS,
    /// Check variable is not null.
    ChkNullV,
    /// Check stack value is not null.
    ChkNullS,
    /// Push object type info.
    OBJTYPE,
    /// Push type ID.
    TYPEID,
    /// Allocate and construct object.
    ALLOC,
    /// Release/free object.
    FREE,
    /// Load 'this' pointer + offset into register.
    LoadThisR,
    /// Load reference object property into register.
    LoadRObjR,
    /// Load value object property into register.
    LoadVObjR,

    // ── Memory operations ────────────────────────────────────────────
    /// Copy N bytes from one address to another.
    COPY,
    /// Add signed immediate to stack pointer in register.
    ADDSi,
    /// Load global + read 4 bytes into register.
    LdGRdR4,
    /// Function pointer constant.
    FuncPtr,
    /// String constant.
    STR,

    // ── List/array initialization ────────────────────────────────────
    /// Allocate memory for list buffer.
    AllocMem,
    /// Set list element count.
    SetListSize,
    /// Push list element address.
    PshListElmnt,
    /// Set list element type.
    SetListType,

    // ── Power operations ─────────────────────────────────────────────
    /// Power: int ** int.
    POWi,
    /// Power: uint ** uint.
    POWu,
    /// Power: float ** float.
    POWf,
    /// Power: double ** double.
    POWd,
    /// Power: double ** int.
    POWdi,
    /// Power: int64 ** int64.
    POWi64,
    /// Power: uint64 ** uint64.
    POWu64,

    // ── Miscellaneous ────────────────────────────────────────────────
    /// Suspend execution (for cooperative multitasking / line callbacks).
    SUSPEND,
    /// JIT compiler entry point.
    JitEntry,
    /// This-call optimization (single arg).
    Thiscall1,

    // ── Debug / metadata (not emitted in final bytecode) ─────────────
    /// Try-catch block marker.
    TryBlock,
    /// Variable declaration marker.
    VarDecl,
    /// Scope block begin/end marker.
    Block,
    /// Object initialization info.
    ObjInfo,
    /// Source line number marker.
    LINE,
    /// Jump label.
    LABEL,
}

impl OpCode {
    /// Size of this instruction in 32-bit dwords (including the opcode word).
    pub fn size_dwords(self) -> u32 {
        use OpCode::*;
        match self {
            // 1 dword: opcode only (no args or args packed into opcode word)
            PopPtr | SwapPtr | PshNull | PshRPtr | RDSPtr | PopRPtr | NOT | INCi8 | INCi16
            | INCi | INCf | INCd | INCi64 | DECi8 | DECi16 | DECi | DECf | DECd | DECi64 | RET
            | SUSPEND | BNOT | BNOT64 => 1,

            // 1 dword: word-sized args packed in upper half
            PshV4 | PshV8 | PshVPtr | PSF | SetV1 | SetV2 | ClrVPtr | CpyVtoV4 | CpyVtoV8
            | CpyVtoR4 | CpyVtoR8 | CpyRtoV4 | CpyRtoV8 | WRTV1 | WRTV2 | WRTV4 | WRTV8 | RDR1
            | RDR2 | RDR4 | RDR8 | LDV | VAR | ADDi | SUBi | MULi | DIVi | MODi | DIVu | MODu
            | ADDf | SUBf | MULf | DIVf | MODf | ADDd | SUBd | MULd | DIVd | MODd | ADDi64
            | SUBi64 | MULi64 | DIVi64 | MODi64 | DIVu64 | MODu64 | NEGi | NEGf | NEGd | NEGi64
            | IncVi | DecVi | BAND | BOR | BXOR | BSLL | BSRL | BSRA | BAND64 | BOR64 | BXOR64
            | BSLL64 | BSRL64 | BSRA64 | CMPi | CMPu | CMPf | CMPd | CMPi64 | CMPu64 | CmpPtr
            | TZ | TNZ | TS | TNS | TP | TNP | iTOf | iTOd | fTOi | fTOu | fTOd | dTOi | dTOu
            | dTOf | i64TOi | i64TOf | i64TOd | fTOi64 | dTOi64 | uTOi64 | iTOi64 | uTOf | uTOd
            | fTOu64 | dTOu64 | u64TOf | u64TOd | sbTOi | swTOi | ubTOi | uwTOi | iTOb | iTOw
            | ClrHi | LOADOBJ | STOREOBJ | GETOBJ | GETOBJREF | GETREF | CHKREF | ChkRefS
            | ChkNullV | ChkNullS | COPY | Thiscall1 => 1,

            // 2 dwords: opcode + 1 dword arg
            PshGPtr | PshC4 | PshG4 | SetV4 | SetG4 | CpyVtoG4 | CpyGtoV4 | LDG | PGA | ADDIi
            | SUBIi | MULIi | ADDIf | SUBIf | MULIf | CMPIi | CMPIf | CMPIu | CALL | JMP | JZ
            | JNZ | JS | JNS | JP | JNP | JMPP | JLowZ | JLowNZ | CallPtr | CALLSYS | CALLBND
            | CALLINTF | Cast | REFCPY | RefCpyV | OBJTYPE | TYPEID | FREE | ADDSi | LdGRdR4
            | FuncPtr | STR | AllocMem | SetListSize | PshListElmnt | SetListType | POWi | POWu
            | POWf | POWd | POWdi | POWi64 | POWu64 | JitEntry | LoadThisR | LoadRObjR
            | LoadVObjR => 2,

            // 3 dwords: opcode + 2 dword args (or 1 qword)
            PshC8 | SetV8 | ALLOC => 3,

            // Debug/metadata (not in final bytecode)
            TryBlock | VarDecl | Block | ObjInfo | LINE | LABEL => 1,
        }
    }

    /// Stack effect in dwords (positive = push, negative = pop).
    /// Returns 0 for variable-effect instructions (context-dependent).
    pub fn stack_delta(self) -> i32 {
        use OpCode::*;
        match self {
            // These push one dword-sized value
            PshC4 | PshV4 | PshG4 | PshNull | PshRPtr => 1,
            // These push two dwords (64-bit)
            PshC8 | PshV8 => 2,
            // These push a pointer
            PshGPtr | PshVPtr | PSF | PGA | RDSPtr | FuncPtr => {
                (std::mem::size_of::<usize>() / 4) as i32
            }
            // These pop a pointer
            PopPtr | PopRPtr => -((std::mem::size_of::<usize>() / 4) as i32),
            // Most operations consume and produce on stack — net 0 or context-dependent
            _ => 0,
        }
    }
}

/// Encoded bytecode instruction with arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub op: OpCode,
    /// First word argument (16-bit).
    pub w_arg0: i16,
    /// Second word argument (16-bit).
    pub w_arg1: i16,
    /// Third word argument (16-bit).
    pub w_arg2: i16,
    /// Dword argument (32-bit).
    pub dw_arg: u32,
    /// Qword argument (64-bit).
    pub qw_arg: u64,
}

impl Instruction {
    /// Create a no-argument instruction.
    pub fn no_arg(op: OpCode) -> Self {
        Instruction {
            op,
            w_arg0: 0,
            w_arg1: 0,
            w_arg2: 0,
            dw_arg: 0,
            qw_arg: 0,
        }
    }

    /// Create an instruction with a single word argument.
    pub fn with_w(op: OpCode, w0: i16) -> Self {
        Instruction {
            op,
            w_arg0: w0,
            w_arg1: 0,
            w_arg2: 0,
            dw_arg: 0,
            qw_arg: 0,
        }
    }

    /// Create an instruction with two word arguments.
    pub fn with_ww(op: OpCode, w0: i16, w1: i16) -> Self {
        Instruction {
            op,
            w_arg0: w0,
            w_arg1: w1,
            w_arg2: 0,
            dw_arg: 0,
            qw_arg: 0,
        }
    }

    /// Create an instruction with three word arguments.
    pub fn with_www(op: OpCode, w0: i16, w1: i16, w2: i16) -> Self {
        Instruction {
            op,
            w_arg0: w0,
            w_arg1: w1,
            w_arg2: w2,
            dw_arg: 0,
            qw_arg: 0,
        }
    }

    /// Create an instruction with a dword argument.
    pub fn with_dw(op: OpCode, dw: u32) -> Self {
        Instruction {
            op,
            w_arg0: 0,
            w_arg1: 0,
            w_arg2: 0,
            dw_arg: dw,
            qw_arg: 0,
        }
    }

    /// Create an instruction with a word + dword argument.
    pub fn with_w_dw(op: OpCode, w0: i16, dw: u32) -> Self {
        Instruction {
            op,
            w_arg0: w0,
            w_arg1: 0,
            w_arg2: 0,
            dw_arg: dw,
            qw_arg: 0,
        }
    }

    /// Create an instruction with a qword argument.
    pub fn with_qw(op: OpCode, qw: u64) -> Self {
        Instruction {
            op,
            w_arg0: 0,
            w_arg1: 0,
            w_arg2: 0,
            dw_arg: 0,
            qw_arg: qw,
        }
    }

    /// Create an instruction with a word + qword argument.
    pub fn with_w_qw(op: OpCode, w0: i16, qw: u64) -> Self {
        Instruction {
            op,
            w_arg0: w0,
            w_arg1: 0,
            w_arg2: 0,
            dw_arg: 0,
            qw_arg: qw,
        }
    }
}

/// Final compiled bytecode buffer.
#[derive(Debug, Clone, Default)]
pub struct ByteCode {
    /// Raw bytecode words.
    pub data: Vec<u32>,
}

impl ByteCode {
    pub fn new() -> Self {
        ByteCode { data: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcode_sizes() {
        assert_eq!(OpCode::PopPtr.size_dwords(), 1);
        assert_eq!(OpCode::PshC4.size_dwords(), 2);
        assert_eq!(OpCode::PshC8.size_dwords(), 3);
        assert_eq!(OpCode::CALL.size_dwords(), 2);
        assert_eq!(OpCode::RET.size_dwords(), 1);
    }

    #[test]
    fn instruction_creation() {
        let inst = Instruction::no_arg(OpCode::RET);
        assert_eq!(inst.op, OpCode::RET);
        assert_eq!(inst.w_arg0, 0);

        let inst = Instruction::with_dw(OpCode::PshC4, 42);
        assert_eq!(inst.op, OpCode::PshC4);
        assert_eq!(inst.dw_arg, 42);

        let inst = Instruction::with_www(OpCode::ADDi, 0, 1, 2);
        assert_eq!(inst.op, OpCode::ADDi);
        assert_eq!(inst.w_arg0, 0);
        assert_eq!(inst.w_arg1, 1);
        assert_eq!(inst.w_arg2, 2);
    }

    #[test]
    fn bytecode_buffer() {
        let mut bc = ByteCode::new();
        assert!(bc.is_empty());
        bc.data.push(0);
        assert_eq!(bc.len(), 1);
    }
}
