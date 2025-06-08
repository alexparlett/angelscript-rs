use crate::core::context::Context;
use crate::core::engine::Engine;
use crate::core::function::Function;
use crate::core::module::Module;
use crate::core::script_generic::ScriptGeneric;
use crate::core::script_object::ScriptObject;
use crate::core::typeinfo::TypeInfo;
use crate::types::enums::MessageType;
use crate::types::script_memory::ScriptMemoryLocation;

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
