use crate::core::context::Context;
use crate::core::engine::Engine;
use crate::core::error::ScriptResult;
use crate::core::function::Function;
use crate::core::module::Module;
use crate::core::script_object::ScriptObject;
use crate::core::typeinfo::TypeInfo;
use crate::internal::utils::read_cstring;
use crate::types::callbacks::{
    CircularRefCallbackFn, CleanContextUserDataCallbackFn, CleanEngineUserDataCallbackFn,
    CleanFunctionUserDataCallbackFn, CleanModuleUserDataCallbackFn, CleanScriptObjectCallbackFn,
    CleanTypeInfoCallbackFn, ExceptionCallbackFn, LineCallbackFn, MessageCallbackFn, MessageInfo,
    RequestContextCallbackFn, ReturnContextCallbackFn, TranslateAppExceptionCallbackFn,
};
use crate::types::enums::MessageType;
use crate::types::script_memory::ScriptMemoryLocation;
use angelscript_sys::{
    asIScriptContext, asIScriptEngine, asIScriptFunction, asIScriptModule, asIScriptObject,
    asITypeInfo, asPWORD, asSMessageInfo,
};
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;
use std::ptr::NonNull;
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

    pub fn set_message_callback(callback: Option<MessageCallbackFn>) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.message_callback = callback;
        Ok(())
    }

    pub fn set_circular_ref_callback(callback: Option<CircularRefCallbackFn>) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.circular_ref_callback = callback;
        Ok(())
    }

    pub fn set_translate_exception_callback(
        callback: Option<TranslateAppExceptionCallbackFn>,
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.translate_exception_callback = callback;
        Ok(())
    }

    pub fn set_request_context_callback(
        callback: Option<RequestContextCallbackFn>,
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.request_context_callback = callback;
        Ok(())
    }

    pub fn set_return_context_callback(
        callback: Option<ReturnContextCallbackFn>,
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.return_context_callback = callback;
        Ok(())
    }

    pub fn set_exception_callback(callback: Option<ExceptionCallbackFn>) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.exception_callback = callback;
        Ok(())
    }

    pub fn set_line_callback(callback: Option<LineCallbackFn>) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager.line_callback = callback;
        Ok(())
    }

    // Engine cleanup callbacks
    pub fn add_engine_user_data_cleanup_callback(
        engine_id: asPWORD,
        callback: CleanEngineUserDataCallbackFn,
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .engine_user_data_cleanup_callbacks
            .insert(engine_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_engine_user_data_cleanup_callback(engine_id: asPWORD) -> ScriptResult<()> {
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
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .module_user_data_cleanup_callbacks
            .insert(module_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_module_user_data_cleanup_callback(module_id: asPWORD) -> ScriptResult<()> {
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
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .context_user_data_cleanup_callbacks
            .insert(context_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_context_user_data_cleanup_callback(context_id: asPWORD) -> ScriptResult<()> {
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
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .function_user_data_cleanup_callbacks
            .insert(function_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_function_user_data_cleanup_callback(function_id: asPWORD) -> ScriptResult<()> {
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
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .type_info_user_data_cleanup_callbacks
            .insert(type_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_type_info_user_data_cleanup_callback(type_id: asPWORD) -> ScriptResult<()> {
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
    ) -> ScriptResult<()> {
        let mut manager = CallbackManager::global().lock()?;
        manager
            .script_object_user_data_cleanup_callbacks
            .insert(object_id, Box::new(callback));
        Ok(())
    }

    pub fn remove_script_object_user_data_cleanup_callback(object_id: asPWORD) -> ScriptResult<()> {
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
            callback(&context, ScriptMemoryLocation::from_const(_params));
        }
    }

    pub unsafe extern "C" fn cvoid_line_callback(ctx: *mut asIScriptContext, params: *mut c_void) {
        if let Some(callback) = CallbackManager::global()
            .lock()
            .ok()
            .and_then(|lock| lock.line_callback)
        {
            let context = Context::from_raw(ctx);
            callback(&context, ScriptMemoryLocation::from_const(params));
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
            if let Some(engine_wrapper) = NonNull::new(engine).map(Engine::from_raw) {
                return match callback(&engine_wrapper) {
                    None => ptr::null_mut(),
                    Some(ctx) => ctx.as_ptr(),
                };
            }
        }
        ptr::null_mut()
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
            if let Some(engine_wrapper) = NonNull::new(engine).map(Engine::from_raw) {
                let ctx_wrapper = Context::from_raw(ctx);
                callback(&engine_wrapper, &ctx_wrapper);
            }
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
                ScriptMemoryLocation::from_const(obj),
                ScriptMemoryLocation::from_mut(params),
            );
        }
    }

    pub unsafe extern "C" fn cvoid_engine_user_data_cleanup_callback(engine: *mut asIScriptEngine) {
        if let Ok(lock) = CallbackManager::global().lock() {
            if let Some(engine_wrapper) = NonNull::new(engine).map(Engine::from_raw) {
                for callback in lock.engine_user_data_cleanup_callbacks.values() {
                    callback(&engine_wrapper);
                }
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
            callback(&wrapper, ScriptMemoryLocation::from_mut(params));
        }
    }
}
