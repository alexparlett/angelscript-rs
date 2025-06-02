// Re-export basic types from raw bindings
pub use crate::raw::{
    asBOOL, asBYTE, asDWORD, asFALSE, asINT32, asINT64, asPWORD, asQWORD, asTRUE, asUINT, asWORD,
};
use crate::{
    asIScriptGeneric, asIScriptGeneric_GetArgDWord, asIScriptGeneric_GetArgDouble,
    asIScriptGeneric_GetArgFloat, asIScriptGeneric_GetArgQWord, asIScriptGeneric_GetArgString,
    read_cstring,
};

// Type aliases for better Rust ergonomics
pub type Byte = asBYTE;
pub type Word = asWORD;
pub type DWord = asDWORD;
pub type QWord = asQWORD;
pub type PWord = asPWORD;
pub type Int = asINT32;
pub type Int64 = asINT64;
pub type UInt = asUINT;
pub type Bool = asBOOL;

pub trait FromScriptGeneric: Sized {
    /// Extract an argument of this type from the context at the given index.
    /// Returns Self or panics if extraction/conversion fails.
    fn from_script_generic(ctx: *mut asIScriptGeneric, arg_idx: u32) -> Self;
}

impl FromScriptGeneric for u32 {
    fn from_script_generic(ctx: *mut asIScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgDWord(ctx, arg_idx) }
    }
}

impl FromScriptGeneric for u64 {
    fn from_script_generic(ctx: *mut asIScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgQWord(ctx, arg_idx) }
    }
}

impl FromScriptGeneric for f32 {
    fn from_script_generic(ctx: *mut asIScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgFloat(ctx, arg_idx) }
    }
}

impl FromScriptGeneric for f64 {
    fn from_script_generic(ctx: *mut asIScriptGeneric, arg_idx: u32) -> Self {
        unsafe { asIScriptGeneric_GetArgDouble(ctx, arg_idx) }
    }
}

// ...and for reference types
impl FromScriptGeneric for &str {
    fn from_script_generic(ctx: *mut asIScriptGeneric, arg_idx: u32) -> Self {
        // Get the char pointer and length
        let c_str_ptr = unsafe { asIScriptGeneric_GetArgString(ctx, arg_idx) };

        // Construct a `&str` from the pointer and length (assumes UTF-8)
        read_cstring(c_str_ptr)
    }
}

// Helper functions
pub fn as_bool(value: bool) -> asBOOL {
    if value { asTRUE } else { asFALSE }
}

pub fn from_as_bool(value: asBOOL) -> bool {
    value != asFALSE
}

pub type GenericFn = unsafe extern "C" fn(*mut asIScriptGeneric);
