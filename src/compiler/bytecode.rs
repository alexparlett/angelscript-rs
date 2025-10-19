use std::fmt;

/// Bytecode instructions that the VM executes
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Push(Value),
    Pop,
    Dup,
    LoadLocal(u32),
    StoreLocal(u32),
    LoadGlobal(u32),
    StoreGlobal(u32),
    LoadMember(u32),
    StoreMember(u32),
    Add, Sub, Mul, Div, Mod, Pow, Neg,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, Not,
    BitAnd, BitOr, BitXor, BitNot, Shl, Shr, UShr,
    Jump(u32),
    JumpIfFalse(u32),
    JumpIfTrue(u32),
    Call(u32, u8),
    CallMethod(u32, u8),
    Return,
    ReturnValue,
    New(u32),
    Delete,
    Cast(u32),
    IsType(u32),
    Index,
    IndexStore,
    Inc, Dec, PreInc, PreDec, PostInc, PostDec,
    Nop,
    Halt,
}

/// Runtime values
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
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
    String(String),
    Handle(u32),
    Null,
}

/// Compiled bytecode module
#[derive(Debug, Clone)]
pub struct BytecodeModule {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub functions: Vec<FunctionInfo>,
    pub types: Vec<TypeInfo>,
    pub globals: Vec<GlobalVar>,
    pub strings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub address: u32,
    pub param_count: u8,
    pub local_count: u32,
    pub return_type: u32,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub size: u32,
    pub members: Vec<MemberInfo>,
    pub methods: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub offset: u32,
    pub type_id: u32,
}

#[derive(Debug, Clone)]
pub struct GlobalVar {
    pub name: String,
    pub type_id: u32,
    pub address: u32,
}

impl BytecodeModule {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            globals: Vec::new(),
            strings: Vec::new(),
        }
    }
}

impl Default for BytecodeModule {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Instruction::Push(v) => write!(f, "PUSH {:?}", v),
            Instruction::Pop => write!(f, "POP"),
            Instruction::Dup => write!(f, "DUP"),
            Instruction::LoadLocal(idx) => write!(f, "LOAD_LOCAL {}", idx),
            Instruction::StoreLocal(idx) => write!(f, "STORE_LOCAL {}", idx),
            Instruction::LoadGlobal(idx) => write!(f, "LOAD_GLOBAL {}", idx),
            Instruction::StoreGlobal(idx) => write!(f, "STORE_GLOBAL {}", idx),
            Instruction::Add => write!(f, "ADD"),
            Instruction::Sub => write!(f, "SUB"),
            Instruction::Mul => write!(f, "MUL"),
            Instruction::Div => write!(f, "DIV"),
            Instruction::Mod => write!(f, "MOD"),
            Instruction::Neg => write!(f, "NEG"),
            Instruction::Eq => write!(f, "EQ"),
            Instruction::Ne => write!(f, "NE"),
            Instruction::Lt => write!(f, "LT"),
            Instruction::Le => write!(f, "LE"),
            Instruction::Gt => write!(f, "GT"),
            Instruction::Ge => write!(f, "GE"),
            Instruction::And => write!(f, "AND"),
            Instruction::Or => write!(f, "OR"),
            Instruction::Not => write!(f, "NOT"),
            Instruction::Jump(addr) => write!(f, "JUMP {}", addr),
            Instruction::JumpIfFalse(addr) => write!(f, "JUMP_IF_FALSE {}", addr),
            Instruction::JumpIfTrue(addr) => write!(f, "JUMP_IF_TRUE {}", addr),
            Instruction::Call(id, argc) => write!(f, "CALL {} ({})", id, argc),
            Instruction::Return => write!(f, "RETURN"),
            Instruction::ReturnValue => write!(f, "RETURN_VALUE"),
            Instruction::Halt => write!(f, "HALT"),
            _ => write!(f, "{:?}", self),
        }
    }
}
