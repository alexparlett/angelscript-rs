use crate::core::types::{FunctionId, ScriptValue, TypeId};
use std::collections::HashMap;
use std::fmt;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Alloc {
        type_id: TypeId,
        func_id: FunctionId,
    },

    Free {
        var: u32,
        func_id: FunctionId,
    },

    LoadObj {
        var: u32,
    },

    StoreObj {
        var: u32,
    },

    RefCpy {
        dst: u32,
        src: u32,
    },

    TypeId {
        type_id: TypeId,
    },

    Cast {
        type_id: TypeId,
    },

    FuncPtr {
        func_id: FunctionId,
    },

    ChkRef {
        var: u32,
    },

    ChkRefS,

    GetProperty {
        obj_var: u32,
        prop_name_id: u32,
        dst_var: u32,
    },

    SetProperty {
        obj_var: u32,
        prop_name_id: u32,
        src_var: u32,
    },

    GetThisProperty {
        prop_name_id: u32,
        dst_var: u32,
    },

    SetThisProperty {
        prop_name_id: u32,
        src_var: u32,
    },

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
    },

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

    TZ,
    TNZ,
    TS,
    TNS,
    TP,
    TNP,

    iTOb {
        var: u32,
    },
    iTOw {
        var: u32,
    },
    sbTOi {
        var: u32,
    },
    swTOi {
        var: u32,
    },
    ubTOi {
        var: u32,
    },
    uwTOi {
        var: u32,
    },
    iTOf {
        var: u32,
    },
    fTOi {
        var: u32,
    },
    uTOf {
        var: u32,
    },
    fTOu {
        var: u32,
    },
    dTOi64 {
        var: u32,
    },
    dTOu64 {
        var: u32,
    },
    i64TOd {
        var: u32,
    },
    u64TOd {
        var: u32,
    },
    dTOi {
        var: u32,
    },
    dTOu {
        var: u32,
    },
    dTOf {
        var: u32,
    },
    iTOd {
        var: u32,
    },
    uTOd {
        var: u32,
    },
    fTOd {
        var: u32,
    },
    i64TOi {
        var: u32,
    },
    i64TOf {
        var: u32,
    },
    u64TOf {
        var: u32,
    },
    uTOi64 {
        var: u32,
    },
    iTOi64 {
        var: u32,
    },
    fTOi64 {
        var: u32,
    },
    fTOu64 {
        var: u32,
    },

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

    CALL {
        func_id: FunctionId,
    },
    CALLINTF {
        func_id: FunctionId,
    },
    CALLSYS {
        sys_func_id: FunctionId,
    },
    CallPtr,
    RET {
        stack_size: u16,
    },

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
    },

    SUSPEND,
    Halt,

    SetV {
        var: u32,
        value: ScriptValue,
    },

    CpyV {
        dst: u32,
        src: u32,
    },

    COPY {
        dst: u32,
        src: u32,
    },

    ClrV {
        var: u32,
    },

    CpyVtoR {
        var: u32,
    },

    CpyRtoV {
        var: u32,
    },

    PshC {
        value: ScriptValue,
    },

    PshV {
        var: u32,
    },

    PshNull,

    Pop,

    PopR,

    PshR,

    Swap,

    CpyVtoG {
        global_id: u32,
        var: u32,
    },

    CpyGtoV {
        var: u32,
        global_id: u32,
    },

    SetG {
        global_id: u32,
        value: ScriptValue,
    },

    PshG {
        global_id: u32,
    },

    LdG {
        global_id: u32,
    },

    ChkNull {
        var: u32,
    },

    ChkNullS,

    Str {
        str_id: u32,
    },

    BeginInitList,

    AddToInitList,

    EndInitList {
        element_type: TypeId,
        count: u32,
    },

    Nop,
}

#[derive(Debug, Clone)]
pub struct BytecodeModule {
    pub instructions: Vec<Instruction>,

    pub function_addresses: HashMap<FunctionId, u32>,

    pub strings: Vec<String>,

    pub property_names: HashMap<String, u32>,

    pub debug_info: Option<DebugInfo>,
}

#[derive(Debug, Clone)]
pub struct DebugInfo {
    pub line_numbers: Vec<(u32, usize)>,
    pub source_sections: Vec<String>,
}

impl BytecodeModule {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            function_addresses: HashMap::new(),
            strings: Vec::new(),
            property_names: HashMap::new(),
            debug_info: None,
        }
    }

    pub fn add_string(&mut self, s: String) -> u32 {
        if let Some(pos) = self.strings.iter().position(|existing| existing == &s) {
            return pos as u32;
        }

        let id = self.strings.len() as u32;
        self.strings.push(s);
        id
    }

    pub fn add_property_name(&mut self, name: String) -> u32 {
        if let Some(&id) = self.property_names.get(&name) {
            return id;
        }

        let id = self.add_string(name.clone());
        self.property_names.insert(name, id);
        id
    }

    pub fn set_function_address(&mut self, func_id: FunctionId, address: u32) {
        self.function_addresses.insert(func_id, address);
    }

    pub fn get_function_address(&self, func_id: FunctionId) -> Option<u32> {
        self.function_addresses.get(&func_id).copied()
    }

    pub fn get_property_name(&self, id: u32) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }

    pub fn get_string(&self, id: u32) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }

    pub fn current_address(&self) -> u32 {
        self.instructions.len() as u32
    }

    pub fn emit(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
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

            Instruction::TZ => write!(f, "TZ"),
            Instruction::TNZ => write!(f, "TNZ"),
            Instruction::TS => write!(f, "TS"),
            Instruction::TNS => write!(f, "TNS"),
            Instruction::TP => write!(f, "TP"),
            Instruction::TNP => write!(f, "TNP"),

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

            Instruction::SetV { var, value } => write!(f, "SETV v{}, {:?}", var, value),
            Instruction::CpyV { dst, src } => write!(f, "CPYV v{}, v{}", dst, src),
            Instruction::COPY { dst, src } => write!(f, "COPY v{}, v{}", dst, src),
            Instruction::ClrV { var } => write!(f, "CLRV v{}", var),
            Instruction::CpyVtoR { var } => write!(f, "CPYVTOR v{}", var),
            Instruction::CpyRtoV { var } => write!(f, "CPYRTOV v{}", var),

            Instruction::PshC { value } => write!(f, "PSHC {:?}", value),
            Instruction::PshV { var } => write!(f, "PSHV v{}", var),
            Instruction::PshNull => write!(f, "PSHNULL"),
            Instruction::Pop => write!(f, "POP"),
            Instruction::PopR => write!(f, "POPR"),
            Instruction::PshR => write!(f, "PSHR"),
            Instruction::Swap => write!(f, "SWAP"),

            Instruction::CpyVtoG { global_id, var } => {
                write!(f, "CPYVTOG g{}, v{}", global_id, var)
            }
            Instruction::CpyGtoV { var, global_id } => {
                write!(f, "CPYGTOV v{}, g{}", var, global_id)
            }
            Instruction::SetG { global_id, value } => write!(f, "SETG g{}, {:?}", global_id, value),
            Instruction::PshG { global_id } => write!(f, "PSHG g{}", global_id),
            Instruction::LdG { global_id } => write!(f, "LDG g{}", global_id),

            Instruction::ChkNull { var } => write!(f, "CHKNULL v{}", var),
            Instruction::ChkNullS => write!(f, "CHKNULLS"),

            Instruction::Str { str_id } => write!(f, "STR s{}", str_id),

            Instruction::BeginInitList => write!(f, "BEGININITLIST"),
            Instruction::AddToInitList => write!(f, "ADDTOINITLIST"),
            Instruction::EndInitList {
                element_type,
                count,
            } => {
                write!(f, "ENDINITLIST t{}, {}", element_type, count)
            }

            Instruction::Nop => write!(f, "NOP"),
        }
    }
}

impl Instruction {
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

    pub fn is_call(&self) -> bool {
        matches!(
            self,
            Instruction::CALL { .. }
                | Instruction::CALLINTF { .. }
                | Instruction::CallPtr
                | Instruction::CALLSYS { .. }
        )
    }

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
