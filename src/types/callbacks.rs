use crate::prelude::{Context, Engine, Function, MessageType, Module, ScriptGeneric, ScriptMemoryLocation, ScriptObject, TypeInfo};

// Callback function types
pub type RequestContextCallbackFn = fn(&Engine) -> Option<Context>;
pub type ReturnContextCallbackFn = fn(&Engine, &Context);
pub type CircularRefCallbackFn = fn(&TypeInfo, ScriptMemoryLocation, ScriptMemoryLocation);
pub type TranslateAppExceptionCallbackFn = fn(&Context, ScriptMemoryLocation);
pub type CleanEngineUserDataCallbackFn = fn(&Engine);
pub type CleanModuleUserDataCallbackFn = fn(&Module);
pub type CleanContextUserDataCallbackFn = fn(&Context);
pub type CleanFunctionUserDataCallbackFn = fn(&Function);
pub type CleanTypeInfoCallbackFn = fn(&TypeInfo);
pub type CleanScriptObjectCallbackFn = fn(&ScriptObject);
pub type ExceptionCallbackFn = fn(&Context, ScriptMemoryLocation);
pub type LineCallbackFn = fn(&Context, ScriptMemoryLocation);
pub type GenericFn = fn(&ScriptGeneric);

#[derive(Debug)]
pub struct MessageInfo {
    pub section: String,
    pub row: u32,
    pub col: u32,
    pub msg_type: MessageType,
    pub message: String,
}

pub type MessageCallbackFn = fn(&MessageInfo);