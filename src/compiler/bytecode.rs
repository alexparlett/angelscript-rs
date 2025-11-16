// src/compiler/bytecode.rs - Complete instruction set with unsigned ops and validation

use std::collections::HashMap;
use std::fmt;
use crate::core::types::ScriptValue;

/// AngelScript bytecode instructions for HashMap-based memory model
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // ==================== OBJECT MANAGEMENT ====================
    /// Allocate a new object on the heap
    Alloc {
        type_id: u32,
        func_id: u32, // Constructor to call (0 = none)
    },

    /// Free an object (call destructor, release memory)
    Free {
        var: u32,
        func_id: u32, // Destructor to call (0 = none)
    },

    /// Move object handle from variable to object register
    LoadObj {
        var: u32,
    },

    /// Move object handle from object register to variable
    StoreObj {
        var: u32,
    },

    /// Copy object handle (increments refcount)
    RefCpy {
        dst: u32,
        src: u32,
    },

    /// Push type ID on stack
    TypeId {
        type_id: u32,
    },

    /// Dynamic cast (pops handle, pushes casted handle or null)
    Cast {
        type_id: u32,
    },

    /// Push function pointer on stack
    FuncPtr {
        func_id: u32,
    },

    /// Check that variable contains valid object handle
    ChkRef {
        var: u32,
    },

    /// Check that stack top contains valid object handle
    ChkRefS,

    // ==================== PROPERTY ACCESS (HASHMAP-BASED) ====================
    /// Get property from object by name
    GetProperty {
        obj_var: u32,
        prop_name_id: u32,
        dst_var: u32,
    },

    /// Set property on object by name
    SetProperty {
        obj_var: u32,
        prop_name_id: u32,
        src_var: u32,
    },

    /// Get property from 'this' object
    GetThisProperty {
        prop_name_id: u32,
        dst_var: u32,
    },

    /// Set property on 'this' object
    SetThisProperty {
        prop_name_id: u32,
        src_var: u32,
    },

    // ==================== MATH INSTRUCTIONS ====================

    // Negation (type-specific)
    NEGi {
        var: u32,
    },
    NEGf {
        var: u32,
    },
    NEGd {
        var: u32,
    },
    NEGi64 {
        var: u32,
    },

    // Integer operations (32-bit signed)
    ADDi {
        dst: u32,
        a: u32,
        b: u32,
    },
    SUBi {
        dst: u32,
        a: u32,
        b: u32,
    },
    MULi {
        dst: u32,
        a: u32,
        b: u32,
    },
    DIVi {
        dst: u32,
        a: u32,
        b: u32,
    },
    MODi {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWi {
        dst: u32,
        a: u32,
        b: u32,
    },

    // Unsigned 32-bit operations
    DIVu {
        dst: u32,
        a: u32,
        b: u32,
    },
    MODu {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWu {
        dst: u32,
        a: u32,
        b: u32,
    },

    // Float operations (32-bit)
    ADDf {
        dst: u32,
        a: u32,
        b: u32,
    },
    SUBf {
        dst: u32,
        a: u32,
        b: u32,
    },
    MULf {
        dst: u32,
        a: u32,
        b: u32,
    },
    DIVf {
        dst: u32,
        a: u32,
        b: u32,
    },
    MODf {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWf {
        dst: u32,
        a: u32,
        b: u32,
    },

    // Double operations (64-bit)
    ADDd {
        dst: u32,
        a: u32,
        b: u32,
    },
    SUBd {
        dst: u32,
        a: u32,
        b: u32,
    },
    MULd {
        dst: u32,
        a: u32,
        b: u32,
    },
    DIVd {
        dst: u32,
        a: u32,
        b: u32,
    },
    MODd {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWd {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWdi {
        dst: u32,
        a: u32,
        b: u32,
    }, // double^int (optimized)

    // Int64 operations (signed)
    ADDi64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    SUBi64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    MULi64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    DIVi64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    MODi64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWi64 {
        dst: u32,
        a: u32,
        b: u32,
    },

    // Unsigned 64-bit operations
    DIVu64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    MODu64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    POWu64 {
        dst: u32,
        a: u32,
        b: u32,
    },

    // Math with immediate values
    ADDIi {
        var: u32,
        imm: i32,
    },
    SUBIi {
        var: u32,
        imm: i32,
    },
    MULIi {
        var: u32,
        imm: i32,
    },
    ADDIf {
        var: u32,
        imm: f32,
    },
    SUBIf {
        var: u32,
        imm: f32,
    },
    MULIf {
        var: u32,
        imm: f32,
    },

    // ==================== BITWISE INSTRUCTIONS ====================
    NOT {
        var: u32,
    },
    BNOT {
        var: u32,
    },
    BNOT64 {
        var: u32,
    },
    BAND {
        dst: u32,
        a: u32,
        b: u32,
    },
    BOR {
        dst: u32,
        a: u32,
        b: u32,
    },
    BXOR {
        dst: u32,
        a: u32,
        b: u32,
    },
    BAND64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    BOR64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    BXOR64 {
        dst: u32,
        a: u32,
        b: u32,
    },
    BSLL {
        dst: u32,
        val: u32,
        shift: u32,
    },
    BSRL {
        dst: u32,
        val: u32,
        shift: u32,
    },
    BSRA {
        dst: u32,
        val: u32,
        shift: u32,
    },
    BSLL64 {
        dst: u32,
        val: u32,
        shift: u32,
    },
    BSRL64 {
        dst: u32,
        val: u32,
        shift: u32,
    },
    BSRA64 {
        dst: u32,
        val: u32,
        shift: u32,
    },

    // ==================== COMPARISON INSTRUCTIONS ====================

    // Compare and store result in value register
    CMPi {
        a: u32,
        b: u32,
    },
    CMPu {
        a: u32,
        b: u32,
    },
    CMPf {
        a: u32,
        b: u32,
    },
    CMPd {
        a: u32,
        b: u32,
    },
    CMPi64 {
        a: u32,
        b: u32,
    },
    CMPu64 {
        a: u32,
        b: u32,
    },
    CmpPtr {
        a: u32,
        b: u32,
    },
    CMPIi {
        var: u32,
        imm: i32,
    },
    CMPIu {
        var: u32,
        imm: u32,
    },
    CMPIf {
        var: u32,
        imm: f32,
    },

    // ==================== TEST INSTRUCTIONS ====================

    // Test value register and update it with boolean result
    TZ,  // Test if zero
    TNZ, // Test if not zero
    TS,  // Test if sign bit set (negative)
    TNS, // Test if sign bit not set (positive or zero)
    TP,  // Test if positive (>0)
    TNP, // Test if not positive (<=0)

    // ==================== TYPE CONVERSION INSTRUCTIONS ====================

    // Conversions (in-place, modifies variable)
    iTOb {
        var: u32,
    }, // int32 to int8
    iTOw {
        var: u32,
    }, // int32 to int16
    sbTOi {
        var: u32,
    }, // int8 to int32 (sign extend)
    swTOi {
        var: u32,
    }, // int16 to int32 (sign extend)
    ubTOi {
        var: u32,
    }, // uint8 to int32 (zero extend)
    uwTOi {
        var: u32,
    }, // uint16 to int32 (zero extend)
    iTOf {
        var: u32,
    }, // int32 to float
    fTOi {
        var: u32,
    }, // float to int32
    uTOf {
        var: u32,
    }, // uint32 to float
    fTOu {
        var: u32,
    }, // float to uint32
    dTOi64 {
        var: u32,
    }, // double to int64
    dTOu64 {
        var: u32,
    }, // double to uint64
    i64TOd {
        var: u32,
    }, // int64 to double
    u64TOd {
        var: u32,
    }, // uint64 to double
    dTOi {
        var: u32,
    }, // double to int32
    dTOu {
        var: u32,
    }, // double to uint32
    dTOf {
        var: u32,
    }, // double to float
    iTOd {
        var: u32,
    }, // int32 to double
    uTOd {
        var: u32,
    }, // uint32 to double
    fTOd {
        var: u32,
    }, // float to double
    i64TOi {
        var: u32,
    }, // int64 to int32 (truncate)
    i64TOf {
        var: u32,
    }, // int64 to float
    u64TOf {
        var: u32,
    }, // uint64 to float
    uTOi64 {
        var: u32,
    }, // uint32 to int64 (zero extend)
    iTOi64 {
        var: u32,
    }, // int32 to int64 (sign extend)
    fTOi64 {
        var: u32,
    }, // float to int64
    fTOu64 {
        var: u32,
    }, // float to uint64

    // ==================== INCREMENT/DECREMENT INSTRUCTIONS ====================
    INCi8 {
        var: u32,
    },
    DECi8 {
        var: u32,
    },
    INCi16 {
        var: u32,
    },
    DECi16 {
        var: u32,
    },
    INCi {
        var: u32,
    },
    DECi {
        var: u32,
    },
    INCi64 {
        var: u32,
    },
    DECi64 {
        var: u32,
    },
    INCf {
        var: u32,
    },
    DECf {
        var: u32,
    },
    INCd {
        var: u32,
    },
    DECd {
        var: u32,
    },

    // ==================== FLOW CONTROL INSTRUCTIONS ====================
    CALL {
        func_id: u32,
    },
    CALLINTF {
        func_id: u32,
    },
    CALLSYS {
        sys_func_id: u32,
    },
    CallPtr, // Call via function pointer on stack
    RET {
        stack_size: u16,
    },

    // Jumps
    JMP {
        offset: i32,
    },
    JZ {
        offset: i32,
    },
    JNZ {
        offset: i32,
    },
    JS {
        offset: i32,
    },
    JNS {
        offset: i32,
    },
    JP {
        offset: i32,
    },
    JNP {
        offset: i32,
    },
    JMPP {
        offset: u32,
    }, // Absolute jump (for switch tables)

    SUSPEND,
    Halt,

    // ==================== VARIABLE OPERATIONS ====================
    /// Set variable to constant value
    SetV {
        var: u32,
        value: ScriptValue,
    },

    /// Copy variable to variable (shallow copy)
    CpyV {
        dst: u32,
        src: u32,
    },

    /// Deep copy for value types (calls opAssign or copies properties)
    COPY {
        dst: u32,
        src: u32,
    },

    /// Clear variable (set to null/void)
    ClrV {
        var: u32,
    },

    /// Copy variable to value register
    CpyVtoR {
        var: u32,
    },

    /// Copy value register to variable
    CpyRtoV {
        var: u32,
    },

    // ==================== STACK OPERATIONS ====================
    /// Push constant on value stack
    PshC {
        value: ScriptValue,
    },

    /// Push variable on value stack
    PshV {
        var: u32,
    },

    /// Push null on value stack
    PshNull,

    /// Pop value from stack (discard)
    Pop,

    /// Pop value from stack to register
    PopR,

    /// Push register to stack
    PshR,

    /// Swap top two values on stack
    Swap,

    // ==================== GLOBAL VARIABLE OPERATIONS ====================
    /// Copy variable to global
    CpyVtoG {
        global_id: u32,
        var: u32,
    },

    /// Copy global to variable
    CpyGtoV {
        var: u32,
        global_id: u32,
    },

    /// Set global to constant
    SetG {
        global_id: u32,
        value: ScriptValue,
    },

    /// Push global on stack
    PshG {
        global_id: u32,
    },

    /// Load global to register
    LdG {
        global_id: u32,
    },

    // ==================== VALIDATION ====================
    /// Check that variable is not null (throw if null)
    ChkNull {
        var: u32,
    },

    /// Check that top of stack is not null
    ChkNullS,

    // ==================== STRING MANAGEMENT ====================
    /// Load string constant to register
    Str {
        str_id: u32,
    },

    // ==================== INITIALIZATION LIST MANAGEMENT ====================
    /// Begin building an initialization list
    BeginInitList,

    /// Add element to current initialization list (pops from value stack)
    AddToInitList,

    /// Finalize initialization list and push on stack
    EndInitList {
        element_type: u32,
        count: u32,
    },

    // ==================== UTILITY ====================
    Nop,
}

// ==================== SCRIPT VALUE ====================

// ==================== BYTECODE MODULE ====================

#[derive(Debug, Clone)]
pub struct BytecodeModule {
    /// Bytecode instructions
    pub instructions: Vec<Instruction>,

    /// Function metadata
    pub functions: Vec<FunctionInfo>,

    /// Type metadata
    pub types: Vec<TypeInfo>,

    /// Global variables
    pub globals: Vec<GlobalVar>,

    /// String constants
    pub strings: Vec<String>,

    /// Property name lookup (property_name -> string_id)
    pub property_names: HashMap<String, u32>,

    /// Debug information (optional)
    pub debug_info: Option<DebugInfo>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub address: u32,
    pub param_count: u8,
    pub local_count: u32,
    pub stack_size: u32,
    pub return_type: u32,
    pub is_script_func: bool,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub members: Vec<MemberInfo>,
    pub methods: Vec<u32>,
    pub flags: TypeFlags,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeFlags: u32 {
        const VALUE_TYPE = 0x01;
        const REF_TYPE = 0x02;
        const HANDLE_TYPE = 0x04;
        const POD = 0x08;
        const HAS_DESTRUCTOR = 0x10;
        const HAS_CONSTRUCTOR = 0x20;
        const ABSTRACT = 0x40;
        const INTERFACE = 0x80;
    }
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub type_id: u32,
    pub flags: MemberFlags,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemberFlags: u32 {
        const PRIVATE = 0x01;
        const PROTECTED = 0x02;
        const PUBLIC = 0x04;
        const CONST = 0x08;
    }
}

#[derive(Debug, Clone)]
pub struct GlobalVar {
    pub name: String,
    pub type_id: u32,
    pub address: u32,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct DebugInfo {
    pub line_numbers: Vec<(u32, usize)>,
    pub source_files: Vec<String>,
    pub local_vars: Vec<(String, u32, u32)>,
}

impl BytecodeModule {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            globals: Vec::new(),
            strings: Vec::new(),
            property_names: HashMap::new(),
            debug_info: None,
        }
    }

    /// Add a string constant and return its ID
    pub fn add_string(&mut self, s: String) -> u32 {
        // Check if string already exists
        if let Some(pos) = self.strings.iter().position(|existing| existing == &s) {
            return pos as u32;
        }

        let id = self.strings.len() as u32;
        self.strings.push(s);
        id
    }

    /// Add a property name and return its ID
    pub fn add_property_name(&mut self, name: String) -> u32 {
        if let Some(&id) = self.property_names.get(&name) {
            return id;
        }

        let id = self.add_string(name.clone());
        self.property_names.insert(name, id);
        id
    }

    /// Find a function by name
    pub fn find_function(&self, name: &str) -> Option<&FunctionInfo> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find a global variable by name
    pub fn find_global(&self, name: &str) -> Option<&GlobalVar> {
        self.globals.iter().find(|g| g.name == name)
    }

    /// Get property name by ID
    pub fn get_property_name(&self, id: u32) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }

    /// Get string by ID
    pub fn get_string(&self, id: u32) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }
}

impl Default for BytecodeModule {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== DISPLAY IMPLEMENTATIONS ====================

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Object management
            Instruction::Alloc { type_id, func_id } => {
                write!(f, "ALLOC t{}, f{}", type_id, func_id)
            }
            Instruction::Free { var, func_id } => write!(f, "FREE v{}, f{}", var, func_id),
            Instruction::LoadObj { var } => write!(f, "LOADOBJ v{}", var),
            Instruction::StoreObj { var } => write!(f, "STOREOBJ v{}", var),
            Instruction::RefCpy { dst, src } => write!(f, "REFCPY v{}, v{}", dst, src),
            Instruction::TypeId { type_id } => write!(f, "TYPEID t{}", type_id),
            Instruction::Cast { type_id } => write!(f, "CAST t{}", type_id),
            Instruction::FuncPtr { func_id } => write!(f, "FUNCPTR f{}", func_id),
            Instruction::ChkRef { var } => write!(f, "CHKREF v{}", var),
            Instruction::ChkRefS => write!(f, "CHKREFS"),

            // Property access
            Instruction::GetProperty {
                obj_var,
                prop_name_id,
                dst_var,
            } => {
                write!(f, "GETPROP v{}, p{}, v{}", obj_var, prop_name_id, dst_var)
            }
            Instruction::SetProperty {
                obj_var,
                prop_name_id,
                src_var,
            } => {
                write!(f, "SETPROP v{}, p{}, v{}", obj_var, prop_name_id, src_var)
            }
            Instruction::GetThisProperty {
                prop_name_id,
                dst_var,
            } => {
                write!(f, "GETTHISPROP p{}, v{}", prop_name_id, dst_var)
            }
            Instruction::SetThisProperty {
                prop_name_id,
                src_var,
            } => {
                write!(f, "SETTHISPROP p{}, v{}", prop_name_id, src_var)
            }

            // Math operations
            Instruction::NEGi { var } => write!(f, "NEGi v{}", var),
            Instruction::NEGf { var } => write!(f, "NEGf v{}", var),
            Instruction::NEGd { var } => write!(f, "NEGd v{}", var),
            Instruction::NEGi64 { var } => write!(f, "NEGi64 v{}", var),

            Instruction::ADDi { dst, a, b } => write!(f, "ADDi v{}, v{}, v{}", dst, a, b),
            Instruction::SUBi { dst, a, b } => write!(f, "SUBi v{}, v{}, v{}", dst, a, b),
            Instruction::MULi { dst, a, b } => write!(f, "MULi v{}, v{}, v{}", dst, a, b),
            Instruction::DIVi { dst, a, b } => write!(f, "DIVi v{}, v{}, v{}", dst, a, b),
            Instruction::MODi { dst, a, b } => write!(f, "MODi v{}, v{}, v{}", dst, a, b),
            Instruction::POWi { dst, a, b } => write!(f, "POWi v{}, v{}, v{}", dst, a, b),

            Instruction::DIVu { dst, a, b } => write!(f, "DIVu v{}, v{}, v{}", dst, a, b),
            Instruction::MODu { dst, a, b } => write!(f, "MODu v{}, v{}, v{}", dst, a, b),
            Instruction::POWu { dst, a, b } => write!(f, "POWu v{}, v{}, v{}", dst, a, b),

            Instruction::ADDf { dst, a, b } => write!(f, "ADDf v{}, v{}, v{}", dst, a, b),
            Instruction::SUBf { dst, a, b } => write!(f, "SUBf v{}, v{}, v{}", dst, a, b),
            Instruction::MULf { dst, a, b } => write!(f, "MULf v{}, v{}, v{}", dst, a, b),
            Instruction::DIVf { dst, a, b } => write!(f, "DIVf v{}, v{}, v{}", dst, a, b),
            Instruction::MODf { dst, a, b } => write!(f, "MODf v{}, v{}, v{}", dst, a, b),
            Instruction::POWf { dst, a, b } => write!(f, "POWf v{}, v{}, v{}", dst, a, b),

            Instruction::ADDd { dst, a, b } => write!(f, "ADDd v{}, v{}, v{}", dst, a, b),
            Instruction::SUBd { dst, a, b } => write!(f, "SUBd v{}, v{}, v{}", dst, a, b),
            Instruction::MULd { dst, a, b } => write!(f, "MULd v{}, v{}, v{}", dst, a, b),
            Instruction::DIVd { dst, a, b } => write!(f, "DIVd v{}, v{}, v{}", dst, a, b),
            Instruction::MODd { dst, a, b } => write!(f, "MODd v{}, v{}, v{}", dst, a, b),
            Instruction::POWd { dst, a, b } => write!(f, "POWd v{}, v{}, v{}", dst, a, b),
            Instruction::POWdi { dst, a, b } => write!(f, "POWdi v{}, v{}, v{}", dst, a, b),

            Instruction::ADDi64 { dst, a, b } => write!(f, "ADDi64 v{}, v{}, v{}", dst, a, b),
            Instruction::SUBi64 { dst, a, b } => write!(f, "SUBi64 v{}, v{}, v{}", dst, a, b),
            Instruction::MULi64 { dst, a, b } => write!(f, "MULi64 v{}, v{}, v{}", dst, a, b),
            Instruction::DIVi64 { dst, a, b } => write!(f, "DIVi64 v{}, v{}, v{}", dst, a, b),
            Instruction::MODi64 { dst, a, b } => write!(f, "MODi64 v{}, v{}, v{}", dst, a, b),
            Instruction::POWi64 { dst, a, b } => write!(f, "POWi64 v{}, v{}, v{}", dst, a, b),

            Instruction::DIVu64 { dst, a, b } => write!(f, "DIVu64 v{}, v{}, v{}", dst, a, b),
            Instruction::MODu64 { dst, a, b } => write!(f, "MODu64 v{}, v{}, v{}", dst, a, b),
            Instruction::POWu64 { dst, a, b } => write!(f, "POWu64 v{}, v{}, v{}", dst, a, b),

            Instruction::ADDIi { var, imm } => write!(f, "ADDIi v{}, {}", var, imm),
            Instruction::SUBIi { var, imm } => write!(f, "SUBIi v{}, {}", var, imm),
            Instruction::MULIi { var, imm } => write!(f, "MULIi v{}, {}", var, imm),
            Instruction::ADDIf { var, imm } => write!(f, "ADDIf v{}, {}", var, imm),
            Instruction::SUBIf { var, imm } => write!(f, "SUBIf v{}, {}", var, imm),
            Instruction::MULIf { var, imm } => write!(f, "MULIf v{}, {}", var, imm),

            // Bitwise
            Instruction::NOT { var } => write!(f, "NOT v{}", var),
            Instruction::BNOT { var } => write!(f, "BNOT v{}", var),
            Instruction::BNOT64 { var } => write!(f, "BNOT64 v{}", var),
            Instruction::BAND { dst, a, b } => write!(f, "BAND v{}, v{}, v{}", dst, a, b),
            Instruction::BOR { dst, a, b } => write!(f, "BOR v{}, v{}, v{}", dst, a, b),
            Instruction::BXOR { dst, a, b } => write!(f, "BXOR v{}, v{}, v{}", dst, a, b),
            Instruction::BAND64 { dst, a, b } => write!(f, "BAND64 v{}, v{}, v{}", dst, a, b),
            Instruction::BOR64 { dst, a, b } => write!(f, "BOR64 v{}, v{}, v{}", dst, a, b),
            Instruction::BXOR64 { dst, a, b } => write!(f, "BXOR64 v{}, v{}, v{}", dst, a, b),
            Instruction::BSLL { dst, val, shift } => {
                write!(f, "BSLL v{}, v{}, v{}", dst, val, shift)
            }
            Instruction::BSRL { dst, val, shift } => {
                write!(f, "BSRL v{}, v{}, v{}", dst, val, shift)
            }
            Instruction::BSRA { dst, val, shift } => {
                write!(f, "BSRA v{}, v{}, v{}", dst, val, shift)
            }
            Instruction::BSLL64 { dst, val, shift } => {
                write!(f, "BSLL64 v{}, v{}, v{}", dst, val, shift)
            }
            Instruction::BSRL64 { dst, val, shift } => {
                write!(f, "BSRL64 v{}, v{}, v{}", dst, val, shift)
            }
            Instruction::BSRA64 { dst, val, shift } => {
                write!(f, "BSRA64 v{}, v{}, v{}", dst, val, shift)
            }

            // Comparisons
            Instruction::CMPi { a, b } => write!(f, "CMPi v{}, v{}", a, b),
            Instruction::CMPu { a, b } => write!(f, "CMPu v{}, v{}", a, b),
            Instruction::CMPf { a, b } => write!(f, "CMPf v{}, v{}", a, b),
            Instruction::CMPd { a, b } => write!(f, "CMPd v{}, v{}", a, b),
            Instruction::CMPi64 { a, b } => write!(f, "CMPi64 v{}, v{}", a, b),
            Instruction::CMPu64 { a, b } => write!(f, "CMPu64 v{}, v{}", a, b),
            Instruction::CmpPtr { a, b } => write!(f, "CMPPtr v{}, v{}", a, b),
            Instruction::CMPIi { var, imm } => write!(f, "CMPIi v{}, {}", var, imm),
            Instruction::CMPIu { var, imm } => write!(f, "CMPIu v{}, {}", var, imm),
            Instruction::CMPIf { var, imm } => write!(f, "CMPIf v{}, {}", var, imm),

            // Tests
            Instruction::TZ => write!(f, "TZ"),
            Instruction::TNZ => write!(f, "TNZ"),
            Instruction::TS => write!(f, "TS"),
            Instruction::TNS => write!(f, "TNS"),
            Instruction::TP => write!(f, "TP"),
            Instruction::TNP => write!(f, "TNP"),

            // Type conversions
            Instruction::iTOb { var } => write!(f, "iTOb v{}", var),
            Instruction::iTOw { var } => write!(f, "iTOw v{}", var),
            Instruction::sbTOi { var } => write!(f, "sbTOi v{}", var),
            Instruction::swTOi { var } => write!(f, "swTOi v{}", var),
            Instruction::ubTOi { var } => write!(f, "ubTOi v{}", var),
            Instruction::uwTOi { var } => write!(f, "uwTOi v{}", var),
            Instruction::iTOf { var } => write!(f, "iTOf v{}", var),
            Instruction::fTOi { var } => write!(f, "fTOi v{}", var),
            Instruction::uTOf { var } => write!(f, "uTOf v{}", var),
            Instruction::fTOu { var } => write!(f, "fTOu v{}", var),
            Instruction::dTOi64 { var } => write!(f, "dTOi64 v{}", var),
            Instruction::dTOu64 { var } => write!(f, "dTOu64 v{}", var),
            Instruction::i64TOd { var } => write!(f, "i64TOd v{}", var),
            Instruction::u64TOd { var } => write!(f, "u64TOd v{}", var),
            Instruction::dTOi { var } => write!(f, "dTOi v{}", var),
            Instruction::dTOu { var } => write!(f, "dTOu v{}", var),
            Instruction::dTOf { var } => write!(f, "dTOf v{}", var),
            Instruction::iTOd { var } => write!(f, "iTOd v{}", var),
            Instruction::uTOd { var } => write!(f, "uTOd v{}", var),
            Instruction::fTOd { var } => write!(f, "fTOd v{}", var),
            Instruction::i64TOi { var } => write!(f, "i64TOi v{}", var),
            Instruction::i64TOf { var } => write!(f, "i64TOf v{}", var),
            Instruction::u64TOf { var } => write!(f, "u64TOf v{}", var),
            Instruction::uTOi64 { var } => write!(f, "uTOi64 v{}", var),
            Instruction::iTOi64 { var } => write!(f, "iTOi64 v{}", var),
            Instruction::fTOi64 { var } => write!(f, "fTOi64 v{}", var),
            Instruction::fTOu64 { var } => write!(f, "fTOu64 v{}", var),

            // Increment/Decrement
            Instruction::INCi8 { var } => write!(f, "INCi8 v{}", var),
            Instruction::DECi8 { var } => write!(f, "DECi8 v{}", var),
            Instruction::INCi16 { var } => write!(f, "INCi16 v{}", var),
            Instruction::DECi16 { var } => write!(f, "DECi16 v{}", var),
            Instruction::INCi { var } => write!(f, "INCi v{}", var),
            Instruction::DECi { var } => write!(f, "DECi v{}", var),
            Instruction::INCi64 { var } => write!(f, "INCi64 v{}", var),
            Instruction::DECi64 { var } => write!(f, "DECi64 v{}", var),
            Instruction::INCf { var } => write!(f, "INCf v{}", var),
            Instruction::DECf { var } => write!(f, "DECf v{}", var),
            Instruction::INCd { var } => write!(f, "INCd v{}", var),
            Instruction::DECd { var } => write!(f, "DECd v{}", var),

            // Flow control
            Instruction::CALL { func_id } => write!(f, "CALL f{}", func_id),
            Instruction::CALLINTF { func_id } => write!(f, "CALLINTF f{}", func_id),
            Instruction::CALLSYS { sys_func_id } => write!(f, "CALLSYS f{}", sys_func_id),
            Instruction::CallPtr => write!(f, "CALLPTR"),
            Instruction::RET { stack_size } => write!(f, "RET {}", stack_size),
            Instruction::JMP { offset } => write!(f, "JMP {:+}", offset),
            Instruction::JZ { offset } => write!(f, "JZ {:+}", offset),
            Instruction::JNZ { offset } => write!(f, "JNZ {:+}", offset),
            Instruction::JS { offset } => write!(f, "JS {:+}", offset),
            Instruction::JNS { offset } => write!(f, "JNS {:+}", offset),
            Instruction::JP { offset } => write!(f, "JP {:+}", offset),
            Instruction::JNP { offset } => write!(f, "JNP {:+}", offset),
            Instruction::JMPP { offset } => write!(f, "JMPP {}", offset),
            Instruction::SUSPEND => write!(f, "SUSPEND"),
            Instruction::Halt => write!(f, "HALT"),

            // Variable operations
            Instruction::SetV { var, value } => write!(f, "SETV v{}, {:?}", var, value),
            Instruction::CpyV { dst, src } => write!(f, "CPYV v{}, v{}", dst, src),
            Instruction::COPY { dst, src } => write!(f, "COPY v{}, v{}", dst, src),
            Instruction::ClrV { var } => write!(f, "CLRV v{}", var),
            Instruction::CpyVtoR { var } => write!(f, "CPYVTOR v{}", var),
            Instruction::CpyRtoV { var } => write!(f, "CPYRTOV v{}", var),

            // Stack operations
            Instruction::PshC { value } => write!(f, "PSHC {:?}", value),
            Instruction::PshV { var } => write!(f, "PSHV v{}", var),
            Instruction::PshNull => write!(f, "PSHNULL"),
            Instruction::Pop => write!(f, "POP"),
            Instruction::PopR => write!(f, "POPR"),
            Instruction::PshR => write!(f, "PSHR"),
            Instruction::Swap => write!(f, "SWAP"),

            // Global operations
            Instruction::CpyVtoG { global_id, var } => {
                write!(f, "CPYVTOG g{}, v{}", global_id, var)
            }
            Instruction::CpyGtoV { var, global_id } => {
                write!(f, "CPYGTOV v{}, g{}", var, global_id)
            }
            Instruction::SetG { global_id, value } => write!(f, "SETG g{}, {:?}", global_id, value),
            Instruction::PshG { global_id } => write!(f, "PSHG g{}", global_id),
            Instruction::LdG { global_id } => write!(f, "LDG g{}", global_id),

            // Validation
            Instruction::ChkNull { var } => write!(f, "CHKNULL v{}", var),
            Instruction::ChkNullS => write!(f, "CHKNULLS"),

            // String
            Instruction::Str { str_id } => write!(f, "STR s{}", str_id),

            // Init lists
            Instruction::BeginInitList => write!(f, "BEGININITLIST"),
            Instruction::AddToInitList => write!(f, "ADDTOINITLIST"),
            Instruction::EndInitList {
                element_type,
                count,
            } => {
                write!(f, "ENDINITLIST t{}, {}", element_type, count)
            }

            // Utility
            Instruction::Nop => write!(f, "NOP"),
        }
    }
}

impl Instruction {
    /// Check if this instruction is a jump instruction
    pub fn is_jump(&self) -> bool {
        matches!(
            self,
            Instruction::JMP { .. }
                | Instruction::JZ { .. }
                | Instruction::JNZ { .. }
                | Instruction::JS { .. }
                | Instruction::JNS { .. }
                | Instruction::JP { .. }
                | Instruction::JNP { .. }
                | Instruction::JMPP { .. }
        )
    }

    /// Check if this instruction is a call instruction
    pub fn is_call(&self) -> bool {
        matches!(
            self,
            Instruction::CALL { .. }
                | Instruction::CALLINTF { .. }
                | Instruction::CallPtr
                | Instruction::CALLSYS { .. }
        )
    }

    /// Check if this instruction terminates a basic block
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Instruction::RET { .. }
                | Instruction::JMP { .. }
                | Instruction::SUSPEND
                | Instruction::Halt
        ) || self.is_jump()
    }
}
