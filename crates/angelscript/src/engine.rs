use crate::context::Context;
use crate::enums::*;
use crate::error::{Error, Result};
use crate::function::Function;
use crate::module::Module;
use crate::typeinfo::TypeInfo;
use std::ffi::CString;
use std::marker::PhantomData;
use std::os::raw::c_void;

pub struct MessageInfo {
    pub section: String,
    pub row: u32,
    pub col: u32,
    pub msg_type: asEMsgType,
    pub message: String,
}

pub type MessageCallbackFn = fn(MessageInfo);

pub struct Engine {
    engine: *mut asIScriptEngine,
    callback: Option<MessageCallbackFn>,
    _phantom: PhantomData<asIScriptEngine>,
}

impl Engine {
    pub fn new() -> Result<Self> {
        unsafe {
            let engine = asCreateScriptEngine(ANGELSCRIPT_VERSION);
            if engine.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Engine {
                    engine,
                    callback: None,
                    _phantom: PhantomData,
                })
            }
        }
    }

    pub fn add_ref(&self) {
        unsafe {
            asEngine_AddRef(self.engine);
        }
    }

    pub fn release(&self) {
        unsafe {
            asEngine_Release(self.engine);
        }
    }

    // Engine properties
    pub fn set_engine_property(&self, property: EngineProp, value: asPWORD) -> Result<()> {
        unsafe { Error::from_code(asEngine_SetEngineProperty(self.engine, property, value)) }
    }

    pub fn get_engine_property(&self, property: EngineProp) -> asPWORD {
        unsafe { asEngine_GetEngineProperty(self.engine, property) }
    }

    unsafe extern "C" fn cvoid_msg_callback(msg_ptr: *const asSMessageInfo, params: *const c_void) {
        let c_msg = msg_ptr.as_ref().expect("asSMessageInfo null");
        let _c_eng = params.as_ref().expect("engine params null");

        let script_engine: &mut Engine = &mut *(params as *mut Engine);

        if let Some(callback) = script_engine.callback {
            let info = MessageInfo {
                section: read_cstring(c_msg.section).to_string(),
                row: c_msg.row as u32,
                col: c_msg.col as u32,
                msg_type: c_msg.type_,
                message: read_cstring(c_msg.message).to_string(),
            };

            callback(info);
        }
    }

    // Message callback
    pub fn set_message_callback(&mut self, callback: MessageCallbackFn) -> Result<()> {
        self.callback = Some(callback);

        type InternalCallback = Option<unsafe extern "C" fn(*const asSMessageInfo, *const c_void)>;
        let base_func: InternalCallback = Some(Engine::cvoid_msg_callback);
        let c_func = unsafe { std::mem::transmute::<InternalCallback, asFUNCTION_t>(base_func) };
        let c_self: *mut c_void = self as *mut _ as *mut c_void;

        unsafe {
            Error::from_code(asEngine_SetMessageCallback(
                self.engine,
                c_func,
                c_self,
                CallConvTypes::asCALL_CDECL as asDWORD,
            ))
        }
    }

    pub fn clear_message_callback(&mut self) -> Result<()> {
        self.callback = None;

        unsafe { Error::from_code(asEngine_ClearMessageCallback(self.engine)) }
    }

    pub fn write_message(
        &self,
        section: &str,
        row: i32,
        col: i32,
        msg_type: i32,
        message: &str,
    ) -> Result<()> {
        let c_section = CString::new(section)?;
        let c_message = CString::new(message)?;

        unsafe {
            Error::from_code(asEngine_WriteMessage(
                self.engine,
                c_section.as_ptr(),
                row,
                col,
                msg_type,
                c_message.as_ptr(),
            ))
        }
    }

    // Global functions
    pub fn register_global_function(
        &self,
        declaration: &str,
        func: GenericFn,
        call_conv: CallConvTypes,
    ) -> Result<()>
    {
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code(asEngine_RegisterGlobalFunction(
                self.engine,
                c_decl.as_ptr(),
                Some(func),
                call_conv as asDWORD,
            ))
        }
    }

    pub fn get_global_function_count(&self) -> asUINT {
        unsafe { asEngine_GetGlobalFunctionCount(self.engine) }
    }

    pub fn get_global_function_by_index(&self, index: asUINT) -> Option<Function> {
        unsafe {
            let func = asEngine_GetGlobalFunctionByIndex(self.engine, index);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_global_function_by_decl(&self, decl: &str) -> Result<Function> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let func = asEngine_GetGlobalFunctionByDecl(self.engine, c_decl.as_ptr());
            if func.is_null() {
                Err(Error::NoFunction)
            } else {
                Ok(Function::from_raw(func))
            }
        }
    }

    // Object types
    pub fn register_object_type(&self, name: &str, byte_size: i32, flags: asDWORD) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe {
            Error::from_code(asEngine_RegisterObjectType(
                self.engine,
                c_name.as_ptr(),
                byte_size,
                flags,
            ))
        }
    }

    pub fn register_object_property(
        &self,
        obj: &str,
        declaration: &str,
        byte_offset: i32,
    ) -> Result<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code(asEngine_RegisterObjectProperty(
                self.engine,
                c_obj.as_ptr(),
                c_decl.as_ptr(),
                byte_offset,
            ))
        }
    }

    pub fn register_object_method(
        &self,
        obj: &str,
        declaration: &str,
        func_ptr: *const (),
        call_conv: CallConvTypes,
    ) -> Result<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code(asEngine_RegisterObjectMethod(
                self.engine,
                c_obj.as_ptr(),
                c_decl.as_ptr(),
                Some(std::mem::transmute(func_ptr)),
                call_conv as asDWORD,
            ))
        }
    }

    pub fn register_object_behaviour(
        &self,
        obj: &str,
        behaviour: Behaviours,
        declaration: &str,
        func_ptr: *const (),
        call_conv: CallConvTypes,
    ) -> Result<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code(asEngine_RegisterObjectBehaviour(
                self.engine,
                c_obj.as_ptr(),
                behaviour,
                c_decl.as_ptr(),
                Some(std::mem::transmute(func_ptr)),
                call_conv as asDWORD,
            ))
        }
    }

    // Modules
    pub fn get_module(&self, name: &str, flag: GMFlags) -> Option<Module> {
        let c_name = CString::new(name).ok()?;

        unsafe {
            let module = asEngine_GetModule(self.engine, c_name.as_ptr(), flag);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    pub fn discard_module(&self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe { Error::from_code(asEngine_DiscardModule(self.engine, c_name.as_ptr())) }
    }

    pub fn get_module_count(&self) -> asUINT {
        unsafe { asEngine_GetModuleCount(self.engine) }
    }

    pub fn get_module_by_index(&self, index: asUINT) -> Option<Module> {
        unsafe {
            let module = asEngine_GetModuleByIndex(self.engine, index);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    // Context
    pub fn create_context(&self) -> Result<Context> {
        unsafe {
            let ctx = asEngine_CreateContext(self.engine);
            if ctx.is_null() {
                Err(Error::OutOfMemory)
            } else {
                Ok(Context::from_raw(ctx))
            }
        }
    }

    // Type identification
    pub fn get_type_info_by_name(&self, name: &str) -> Option<TypeInfo> {
        let c_name = CString::new(name).ok()?;

        unsafe {
            let type_info = asEngine_GetTypeInfoByName(self.engine, c_name.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_type_info_by_decl(&self, decl: &str) -> Option<TypeInfo> {
        let c_decl = CString::new(decl).ok()?;

        unsafe {
            let type_info = asEngine_GetTypeInfoByDecl(self.engine, c_decl.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_type_id_by_decl(&self, decl: &str) -> Result<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let type_id = asEngine_GetTypeIdByDecl(self.engine, c_decl.as_ptr());
            if type_id < 0 {
                Error::from_code(type_id)?;
            }
            Ok(type_id)
        }
    }

    pub fn as_ptr(&self) -> *mut asIScriptEngine {
        self.engine
    }

    pub fn register_std(&self) {
        unsafe { asEngine_RegisterStd(self.engine) }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Clear callbacks
        unsafe {
            asEngine_ShutDownAndRelease(self.engine);
        }
    }
}

unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}
