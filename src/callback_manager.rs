use crate::utils::read_cstring;
use crate::{
    Context, Engine, Function, MessageType, Module, Result, ScriptGeneric,
    ScriptObject, TypeInfo, VoidPtr,
};
use angelscript_bindings::{
    asIScriptContext, asIScriptEngine, asIScriptFunction, asIScriptModule, asIScriptObject,
    asITypeInfo, asPWORD, asSMessageInfo,
};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::sync::{Mutex, OnceLock};

static CALLBACK_MANAGER: OnceLock<Mutex<CallbackManager>> = OnceLock::new();

#[derive(Debug)]
pub struct CallbackManager {
    message_callback: Option<MessageCallbackFn>,
    circular_ref_callback: Option<CircularRefCallbackFn>,
    translate_exception_callback: Option<TranslateAppExceptionCallbackFn>,
    request_context_callback: Option<RequestContextCallbackFn>,
    return_context_callback: Option<ReturnContextCallbackFn>,
    exception_callback: Option<ExceptionCallbackFn>,
    line_callback: Option<LineCallbackFn>,
    engine_user_data_cleanup_callbacks: HashMap<asPWORD, Box<CleanEngineUserDataCallbackFn>>,
    module_user_data_cleanup_callbacks: HashMap<asPWORD, Box<CleanModuleUserDataCallbackFn>>,
    context_user_data_cleanup_callbacks: HashMap<asPWORD, Box<CleanContextUserDataCallbackFn>>,
    function_user_data_cleanup_callbacks: HashMap<asPWORD, Box<CleanFunctionUserDataCallbackFn>>,
    type_info_user_data_cleanup_callbacks: HashMap<asPWORD, Box<CleanTypeInfoCallbackFn>>,
    script_object_user_data_cleanup_callbacks: HashMap<asPWORD, Box<CleanScriptObjectCallbackFn>>,
}

impl CallbackManager {
    fn global() -> &'static Mutex<CallbackManager> {
        CALLBACK_MANAGER.get_or_init(|| {
            Mutex::new(CallbackManager {
                message_callback: None,
                circular_ref_callback: None,
                translate_exception_callback: None,
                request_context_callback: None,
                return_context_callback: None,
                exception_callback: None,
                line_callback: None,
                engine_user_data_cleanup_callbacks: HashMap::new(),
                module_user_data_cleanup_callbacks: HashMap::new(),
                context_user_data_cleanup_callbacks: HashMap::new(),
                function_user_data_cleanup_callbacks: HashMap::new(),
                type_info_user_data_cleanup_callbacks: HashMap::new(),
                script_object_user_data_cleanup_callbacks: HashMap::new(),
            })
        })
    }

    pub fn set_message_callback(callback: Option<MessageCallbackFn>) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.message_callback = callback;
        Ok(())
    }

    pub fn set_circular_ref_callback(callback: Option<CircularRefCallbackFn>) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.circular_ref_callback = callback;
        Ok(())
    }

    pub fn set_translate_exception_callback(
        callback: Option<TranslateAppExceptionCallbackFn>,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.translate_exception_callback = callback;
        Ok(())
    }

    pub fn set_request_context_callback(callback: Option<RequestContextCallbackFn>) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.request_context_callback = callback;
        Ok(())
    }

    pub fn set_return_context_callback(callback: Option<ReturnContextCallbackFn>) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.return_context_callback = callback;
        Ok(())
    }

    pub fn set_exception_callback(callback: Option<ExceptionCallbackFn>) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.exception_callback = callback;
        Ok(())
    }

    pub fn set_line_callback(callback: Option<LineCallbackFn>) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.line_callback = callback;
        Ok(())
    }

    // Engine cleanup callbacks
    pub fn add_engine_user_data_cleanup_callback(
        engine_id: asPWORD,
        callback: CleanEngineUserDataCallbackFn,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .engine_user_data_cleanup_callbacks
            .insert(engine_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_engine_user_data_cleanup_callback(engine_id: asPWORD) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .engine_user_data_cleanup_callbacks
            .remove(&engine_id);
        Ok(())
    }

    // Module cleanup callbacks
    pub fn add_module_user_data_cleanup_callback(
        module_id: asPWORD,
        callback: CleanModuleUserDataCallbackFn,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .module_user_data_cleanup_callbacks
            .insert(module_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_module_user_data_cleanup_callback(module_id: asPWORD) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .module_user_data_cleanup_callbacks
            .remove(&module_id);
        Ok(())
    }

    // Context cleanup callbacks
    pub fn add_context_user_data_cleanup_callback(
        context_id: asPWORD,
        callback: CleanContextUserDataCallbackFn,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .context_user_data_cleanup_callbacks
            .insert(context_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_context_user_data_cleanup_callback(context_id: asPWORD) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .context_user_data_cleanup_callbacks
            .remove(&context_id);
        Ok(())
    }

    // Function cleanup callbacks
    pub fn add_function_user_data_cleanup_callback(
        function_id: asPWORD,
        callback: CleanFunctionUserDataCallbackFn,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .function_user_data_cleanup_callbacks
            .insert(function_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_function_user_data_cleanup_callback(function_id: asPWORD) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .function_user_data_cleanup_callbacks
            .remove(&function_id);
        Ok(())
    }

    // Type info cleanup callbacks
    pub fn add_type_info_user_data_cleanup_callback(
        type_id: asPWORD,
        callback: CleanTypeInfoCallbackFn,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .type_info_user_data_cleanup_callbacks
            .insert(type_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_type_info_user_data_cleanup_callback(type_id: asPWORD) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .type_info_user_data_cleanup_callbacks
            .remove(&type_id);
        Ok(())
    }

    // Script object cleanup callbacks
    pub fn add_script_object_user_data_cleanup_callback(
        object_id: asPWORD,
        callback: CleanScriptObjectCallbackFn,
    ) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .script_object_user_data_cleanup_callbacks
            .insert(object_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_script_object_user_data_cleanup_callback(object_id: asPWORD) -> Result<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .script_object_user_data_cleanup_callbacks
            .remove(&object_id);
        Ok(())
    }

    // C trampolines moved here
    pub unsafe extern "C" fn cvoid_exception_callback(
        ctx: *mut asIScriptContext,
        _params: *const c_void,
    ) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.exception_callback)
        {
            let context = Context::from_raw(ctx);
            callback(&context, VoidPtr::from_const_raw(_params));
        }
    }

    pub unsafe extern "C" fn cvoid_line_callback(ctx: *mut asIScriptContext, params: *mut c_void) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.line_callback)
        {
            let context = Context::from_raw(ctx);
            callback(&context, VoidPtr::from_const_raw(params));
        }
    }

    pub unsafe extern "C" fn cvoid_msg_callback(
        msg_ptr: *const asSMessageInfo,
        _params: *mut c_void,
    ) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.message_callback)
        {
            let c_msg = unsafe { msg_ptr.as_ref().expect("Unable to read message") };

            let info = MessageInfo {
                section: read_cstring(c_msg.section).to_string(),
                row: c_msg.row as u32,
                col: c_msg.col as u32,
                msg_type: MessageType::from(c_msg.type_),
                message: read_cstring(c_msg.message).to_string(),
            };

            callback(&info);
        }
    }

    pub unsafe extern "C" fn cvoid_request_context_callback(
        engine: *mut asIScriptEngine,
        _params: *mut std::os::raw::c_void,
    ) -> *mut asIScriptContext {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.request_context_callback)
        {
            let engine_wrapper = Engine::from_raw(engine);
            match callback(&engine_wrapper) {
                None => ptr::null_mut(),
                Some(ctx) => ctx.as_ptr(),
            }
        } else {
            ptr::null_mut()
        }
    }

    // Return context callback
    pub unsafe extern "C" fn cvoid_return_context_callback(
        engine: *mut asIScriptEngine,
        ctx: *mut asIScriptContext,
        _params: *mut std::os::raw::c_void,
    ) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.return_context_callback)
        {
            let engine_wrapper = Engine::from_raw(engine);
            let ctx_wrapper = Context::from_raw(ctx);
            callback(&engine_wrapper, &ctx_wrapper);
        }
    }

    pub unsafe extern "C" fn cvoid_circular_ref_callback(
        type_info: *mut asITypeInfo,
        obj: *const std::os::raw::c_void,
        params: *mut std::os::raw::c_void,
    ) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.circular_ref_callback)
        {
            let type_info_wrapper = TypeInfo::from_raw(type_info);
            callback(
                &type_info_wrapper,
                VoidPtr::from_const_raw(obj),
                VoidPtr::from_mut_raw(params),
            );
        }
    }

    pub unsafe extern "C" fn cvoid_engine_user_data_cleanup_callback(engine: *mut asIScriptEngine) {
        if let Ok(lock) = CallbackManager::global().lock() {
            let engine_wrapper = Engine::from_raw(engine);
            for callback in lock.engine_user_data_cleanup_callbacks.values() {
                callback(&engine_wrapper);
            }
        }
    }

    pub unsafe extern "C" fn cvoid_module_user_data_cleanup_callback(module: *mut asIScriptModule) {
        if let Ok(lock) = CallbackManager::global().lock() {
            let module_wrapper = Module::from_raw(module);
            for callback in lock.module_user_data_cleanup_callbacks.values() {
                callback(&module_wrapper);
            }
        }
    }

    pub unsafe extern "C" fn cvoid_context_user_data_cleanup_callback(
        context: *mut asIScriptContext,
    ) {
        if let Ok(lock) = CallbackManager::global().lock() {
            let wrapper = Context::from_raw(context);
            for callback in lock.context_user_data_cleanup_callbacks.values() {
                callback(&wrapper);
            }
        }
    }

    pub unsafe extern "C" fn cvoid_function_user_data_cleanup_callback(
        function: *mut asIScriptFunction,
    ) {
        if let Ok(lock) = CallbackManager::global().lock() {
            let wrapper = Function::from_raw(function);
            for callback in lock.function_user_data_cleanup_callbacks.values() {
                callback(&wrapper);
            }
        }
    }

    pub unsafe extern "C" fn cvoid_type_info_user_data_cleanup_callback(
        type_info: *mut asITypeInfo,
    ) {
        if let Ok(lock) = CallbackManager::global().lock() {
            let wrapper = TypeInfo::from_raw(type_info);
            for callback in lock.type_info_user_data_cleanup_callbacks.values() {
                callback(&wrapper);
            }
        }
    }

    pub unsafe extern "C" fn cvoid_script_object_user_data_cleanup_callback(
        script_object: *mut asIScriptObject,
    ) {
        if let Ok(lock) = CallbackManager::global().lock() {
            let wrapper = ScriptObject::from_raw(script_object);
            for callback in lock.script_object_user_data_cleanup_callbacks.values() {
                callback(&wrapper);
            }
        }
    }

    pub unsafe extern "C" fn cvoid_translate_exception_callback(
        ctx: *mut asIScriptContext,
        params: *mut std::os::raw::c_void,
    ) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.translate_exception_callback)
        {
            let wrapper = Context::from_raw(ctx);
            callback(&wrapper, VoidPtr::from_mut_raw(params));
        }
    }
}

#[derive(Debug)]
pub struct MessageInfo {
    pub section: String,
    pub row: u32,
    pub col: u32,
    pub msg_type: MessageType,
    pub message: String,
}

pub type MessageCallbackFn = fn(&MessageInfo);

// Callback function types
pub type RequestContextCallbackFn = fn(&Engine) -> Option<Context>;
pub type ReturnContextCallbackFn = fn(&Engine, &Context);
pub type CircularRefCallbackFn = fn(&TypeInfo, VoidPtr, VoidPtr);
pub type TranslateAppExceptionCallbackFn = fn(&Context, VoidPtr);
pub type CleanEngineUserDataCallbackFn = fn(&Engine);
pub type CleanModuleUserDataCallbackFn = fn(&Module);
pub type CleanContextUserDataCallbackFn = fn(&Context);
pub type CleanFunctionUserDataCallbackFn = fn(&Function);
pub type CleanTypeInfoCallbackFn = fn(&TypeInfo);
pub type CleanScriptObjectCallbackFn = fn(&ScriptObject);
pub type ExceptionCallbackFn = fn(&Context, VoidPtr);
pub type LineCallbackFn = fn(&Context, VoidPtr);
pub type GenericFn = fn(&ScriptGeneric);
