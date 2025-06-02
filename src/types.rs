// Re-export basic types from raw bindings
use crate::ffi::{
    asILockableSharedBool, asIScriptGeneric_GetArgDWord, asIScriptGeneric_GetArgDouble,
    asIScriptGeneric_GetArgFloat, asIScriptGeneric_GetArgQWord, asIScriptGeneric_GetArgString,
};
use crate::scriptgeneric::ScriptGeneric;
use crate::{utils, MsgType};
use angelscript_bindings::asIScriptGeneric;

pub trait FromScriptGeneric: Sized {
    /// Extract an argument of this type from the context at the given index.
    /// Returns Self or panics if extraction/conversion fails.
    fn from_script_generic(ctx: &ScriptGeneric, arg_idx: u32) -> Self;
}

impl FromScriptGeneric for u32 {
    fn from_script_generic(ctx: &ScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgDWord(ctx.as_ptr(), arg_idx) }
    }
}

impl FromScriptGeneric for u64 {
    fn from_script_generic(ctx: &ScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgQWord(ctx.as_ptr(), arg_idx) }
    }
}

impl FromScriptGeneric for f32 {
    fn from_script_generic(ctx: &ScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgFloat(ctx.as_ptr(), arg_idx) }
    }
}

impl FromScriptGeneric for f64 {
    fn from_script_generic(ctx: &ScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgDouble(ctx.as_ptr(), arg_idx) }
    }
}

// ...and for reference types
impl FromScriptGeneric for &str {
    fn from_script_generic(ctx: &ScriptGeneric, arg_idx: u32) -> Self {
        // Get the char pointer and length
        let c_str_ptr = unsafe { asIScriptGeneric_GetArgString(ctx.as_ptr(), arg_idx) };

        // Construct a `&str` from the pointer and length (assumes UTF-8)
        utils::read_cstring(c_str_ptr)
    }
}

pub trait UserData {
    const TypeId: usize;
}

pub struct WeakRef(pub(crate) *mut asILockableSharedBool);

pub struct MessageInfo {
    pub section: String,
    pub row: u32,
    pub col: u32,
    pub msg_type: MsgType,
    pub message: String,
}

pub type MessageCallbackFn = fn(crate::MessageInfo);

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub type_id: i32,
    pub flags: u32,
    pub name: Option<&'static str>,
    pub default_arg: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: Option<&'static str>,
    pub type_id: i32,
}

#[derive(Debug, Clone)]
pub struct GlobalVarInfo {
    pub name: &'static str,
    pub namespace: &'static str,
    pub type_id: i32,
    pub is_const: bool,
}

pub(crate) type GenericFn = fn(&ScriptGeneric);

pub(crate) struct GenericFnUserData(pub GenericFn);

impl GenericFnUserData {
    pub fn call(& self, ctx: &ScriptGeneric) {
        (self.0)(ctx)
    }
}

impl UserData for GenericFnUserData {
    const TypeId: usize = 0x129032719; // Must be unique!
}


pub type ScriptGenericFn = unsafe extern "C" fn(ctx: *mut asIScriptGeneric);
