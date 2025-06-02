// Since enums are now defined in angelscript.h and included via bindgen,
// we just re-export them from the raw bindings

// Re-export enums from raw bindings
pub use crate::raw::{
    asEBCInstr as BCInstr, asEBCType as BCType, asEBehaviours as Behaviours,
    asECallConvTypes as CallConvTypes, asECompileFlags as CompileFlags,
    asEContextState as ContextState, asEEngineProp as EngineProp, asEFuncType as FuncType,
    asEGCFlags as GCFlags, asEGMFlags as GMFlags, asEMsgType as MsgType,
    asEObjTypeFlags as ObjTypeFlags, asERetCodes as RetCodes, asETokenClass as TokenClass,
    asETypeIdFlags as TypeIdFlags, asETypeModifiers as TypeModifiers,
};
pub use crate::raw::*;