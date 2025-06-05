use crate::callback_manager::{
    CallbackManager, CircularRefCallbackFn, CleanContextUserDataCallbackFn,
    CleanEngineUserDataCallbackFn, CleanFunctionUserDataCallbackFn, CleanModuleUserDataCallbackFn,
    CleanScriptObjectCallbackFn, CleanTypeInfoCallbackFn, GenericFn, MessageCallbackFn,
    RequestContextCallbackFn, ReturnContextCallbackFn, TranslateAppExceptionCallbackFn,
};
use crate::context::Context;
use crate::enums::*;
use crate::error::{Error, Result};
use crate::ffi::{asGetLibraryOptions, asGetLibraryVersion, asIScriptEngine};
use crate::function::Function;
use crate::jit_compiler::JITCompiler;
use crate::module::Module;
use crate::string::with_string_module;
use crate::typeinfo::TypeInfo;
use crate::user_data::UserData;
use crate::{LockableSharedBool, Ptr, ScriptGeneric};
use angelscript_bindings::{
    asDWORD, asECallConvTypes_asCALL_GENERIC, asEMsgType, asGENFUNC_t, asGenericFunction,
    asIScriptEngine__bindgen_vtable, asIScriptGeneric, asIStringFactory, asITypeInfo,
    asMessageInfoFunction, asPWORD, asScriptContextFunction, asUINT,
};
use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_void;
use std::ptr;

#[derive(Debug, PartialEq, Eq)]
pub struct Engine {
    inner: *mut asIScriptEngine,
    is_root: bool,
}

impl Engine {
    pub(crate) fn from_raw(engine: *mut asIScriptEngine) -> Self {
        let engine_wrapper = Engine {
            inner: engine,
            is_root: false,
        };
        engine_wrapper
            .add_ref()
            .expect("Failed to add ref to engine");
        engine_wrapper
    }

    pub(crate) fn new(engine_ptr: *mut asIScriptEngine) -> Result<Self> {
        unsafe {
            if engine_ptr.is_null() {
                Err(Error::FailedToCreateEngine)
            } else {
                let engine_wrapper = Engine {
                    inner: engine_ptr,
                    is_root: true,
                };
                Ok(engine_wrapper)
            }
        }
    }

    pub fn get_library_version() -> &'static str {
        unsafe {
            let version = asGetLibraryVersion();
            if version.is_null() {
                ""
            } else {
                CStr::from_ptr(version).to_str().unwrap_or("")
            }
        }
    }

    pub fn get_library_options() -> &'static str {
        unsafe {
            let options = asGetLibraryOptions();
            if options.is_null() {
                ""
            } else {
                CStr::from_ptr(options).to_str().unwrap_or("")
            }
        }
    }

    // Reference counting (following bindings order)
    pub fn add_ref(&self) -> Result<()> {
        unsafe { Error::from_code((self.as_vtable().asIScriptEngine_AddRef)(self.inner)) }
    }

    pub fn release(&self) -> Result<()> {
        unsafe { Error::from_code((self.as_vtable().asIScriptEngine_Release)(self.inner)) }
    }

    pub fn shutdown_and_release(&self) -> Result<()> {
        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_ShutDownAndRelease)(
                self.inner,
            ))
        }
    }

    // Engine properties
    pub fn set_engine_property(&self, property: EngineProperty, value: usize) -> Result<()> {
        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_SetEngineProperty)(
                self.inner,
                property.into(),
                value,
            ))
        }
    }

    pub fn get_engine_property(&self, property: EngineProperty) -> asPWORD {
        unsafe { (self.as_vtable().asIScriptEngine_GetEngineProperty)(self.inner, property.into()) }
    }

    pub fn set_message_callback(&mut self, callback: MessageCallbackFn) -> Result<()> {
        CallbackManager::set_message_callback(Some(callback))?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_SetMessageCallback)(
                self.inner,
                &mut asMessageInfoFunction(Some(CallbackManager::cvoid_msg_callback)),
                ptr::null_mut(),
                CallingConvention::Cdecl.into(),
            ))
        }
    }

    pub fn clear_message_callback(&mut self) -> Result<()> {
        CallbackManager::set_message_callback(None)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_ClearMessageCallback)(
                self.inner,
            ))
        }
    }

    pub fn write_message(
        &self,
        section: &str,
        row: i32,
        col: i32,
        msg_type: asEMsgType,
        message: &str,
    ) -> Result<()> {
        let c_section = CString::new(section)?;
        let c_message = CString::new(message)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_WriteMessage)(
                self.inner,
                c_section.as_ptr(),
                row,
                col,
                msg_type,
                c_message.as_ptr(),
            ))
        }
    }

    // JIT compiler
    pub fn set_jit_compiler(&self, compiler: &mut JITCompiler) -> Result<()> {
        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_SetJITCompiler)(
                self.inner,
                compiler.as_ptr(),
            ))
        }
    }

    pub fn get_jit_compiler(&self) -> Option<JITCompiler> {
        unsafe {
            let compiler = (self.as_vtable().asIScriptEngine_GetJITCompiler)(self.inner);
            if compiler.is_null() {
                None
            } else {
                Some(JITCompiler::from_raw(compiler))
            }
        }
    }

    // Global functions
    unsafe extern "C" fn generic_callback_thunk(arg1: *mut asIScriptGeneric) {
        let func = ScriptGeneric::from_raw(arg1).get_function();
        let user_data = func.get_user_data::<GenericFnUserData>();

        if let Some(f) = user_data {
            let s = ScriptGeneric::from_raw(arg1);
            f.as_ref().call(&s);
        }
    }

    pub fn register_global_function<T>(
        &self,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&mut T>,
    ) -> Result<()> {
        let c_decl = CString::new(declaration)?;

        let base_func: asGENFUNC_t = Some(Engine::generic_callback_thunk);

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterGlobalFunction)(
                self.inner,
                c_decl.as_ptr(),
                &mut asGenericFunction(base_func),
                CallingConvention::Generic.into(),
                auxiliary
                    .map(|aux| aux as *mut _ as *mut c_void)
                    .unwrap_or_else(|| std::ptr::null_mut()),
            ))
        }?;

        if let Some(func_obj) = self.get_global_function_by_decl(declaration) {
            let user_data = Box::new(GenericFnUserData(func_ptr));
            func_obj.set_user_data(Box::leak(user_data));
        }

        Ok(())
    }

    pub fn get_global_function_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetGlobalFunctionCount)(self.inner) }
    }

    pub fn get_global_function_by_index(&self, index: u32) -> Option<Function> {
        unsafe {
            let func =
                (self.as_vtable().asIScriptEngine_GetGlobalFunctionByIndex)(self.inner, index);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_global_function_by_decl(&self, decl: &str) -> Option<Function> {
        let c_decl = CString::new(decl).ok()?;

        unsafe {
            let func = (self.as_vtable().asIScriptEngine_GetGlobalFunctionByDecl)(
                self.inner,
                c_decl.as_ptr(),
            );
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // Global properties
    pub fn register_global_property<T>(&self, declaration: &str, pointer: &mut T) -> Result<()> {
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterGlobalProperty)(
                self.inner,
                c_decl.as_ptr(),
                pointer as *mut _ as *mut c_void,
            ))
        }
    }

    pub fn get_global_property_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetGlobalPropertyCount)(self.inner) }
    }

    pub fn get_global_property_by_index(&self, index: asUINT) -> Result<GlobalPropertyInfo> {
        let mut name: *const c_char = ptr::null();
        let mut name_space: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut is_const: bool = false;
        let mut config_group: *const c_char = ptr::null();
        let mut pointer: *mut c_void = ptr::null_mut();
        let mut access_mask: asDWORD = 0;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_GetGlobalPropertyByIndex)(
                self.inner,
                index,
                &mut name,
                &mut name_space,
                &mut type_id,
                &mut is_const,
                &mut config_group,
                &mut pointer,
                &mut access_mask,
            ))?;

            Ok(GlobalPropertyInfo {
                name: if name.is_null() {
                    None
                } else {
                    CStr::from_ptr(name).to_str().ok().map(|s| s.to_string())
                },
                name_space: if name_space.is_null() {
                    None
                } else {
                    CStr::from_ptr(name_space)
                        .to_str()
                        .ok()
                        .map(|s| s.to_string())
                },
                type_id,
                is_const,
                config_group: if config_group.is_null() {
                    None
                } else {
                    CStr::from_ptr(config_group)
                        .to_str()
                        .ok()
                        .map(|s| s.to_string())
                },
                pointer: Ptr::<c_void>::from_raw(pointer),
                access_mask,
            })
        }
    }

    pub fn get_global_property_index_by_name(&self, name: &str) -> Result<i32> {
        let c_name = CString::new(name)?;

        unsafe {
            let index = (self
                .as_vtable()
                .asIScriptEngine_GetGlobalPropertyIndexByName)(
                self.inner, c_name.as_ptr()
            );
            if index < 0 {
                Error::from_code(index)?;
            }
            Ok(index)
        }
    }

    pub fn get_global_property_index_by_decl(&self, decl: &str) -> Result<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let index = (self
                .as_vtable()
                .asIScriptEngine_GetGlobalPropertyIndexByDecl)(
                self.inner, c_decl.as_ptr()
            );
            if index < 0 {
                Error::from_code(index)?;
            }
            Ok(index)
        }
    }

    // Object types
    pub fn register_object_type(
        &self,
        name: &str,
        byte_size: usize,
        flags: ObjectTypeFlags,
    ) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterObjectType)(
                self.inner,
                c_name.as_ptr(),
                byte_size as i32,
                flags.into(),
            ))
        }
    }

    pub fn register_object_property(
        &self,
        obj: &str,
        declaration: &str,
        byte_offset: i32,
        composite_offset: i32,
        is_composite_indirect: bool,
    ) -> Result<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterObjectProperty)(
                self.inner,
                c_obj.as_ptr(),
                c_decl.as_ptr(),
                byte_offset,
                composite_offset,
                is_composite_indirect,
            ))
        }
    }

    pub fn register_object_method<T>(
        &self,
        obj: &str,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&mut T>,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> Result<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        let base_func: asGENFUNC_t = Some(Engine::generic_callback_thunk);

        let func_id = unsafe {
            let result = (self.as_vtable().asIScriptEngine_RegisterObjectMethod)(
                self.inner,
                c_obj.as_ptr(),
                c_decl.as_ptr(),
                &mut asGenericFunction(base_func),
                CallingConvention::Generic.into(),
                auxiliary
                    .map(|aux| aux as *mut _ as *mut c_void)
                    .unwrap_or_else(|| std::ptr::null_mut()),
                composite_offset.unwrap_or_else(|| 0),
                is_composite_indirect.unwrap_or_else(|| false),
            );

            Error::from_code(result).map(|_| result)
        }?;

        if let Some(func_obj) = self.get_function_by_id(func_id) {
            let user_data = Box::new(GenericFnUserData(func_ptr));
            func_obj.set_user_data(Box::leak(user_data));
        }

        Ok(())
    }

    pub fn register_object_behaviour<T>(
        &self,
        obj: &str,
        behaviour: Behaviour,
        declaration: &str,
        func_ptr: GenericFn,
        auxiliary: Option<&mut T>,
        composite_offset: Option<i32>,
        is_composite_indirect: Option<bool>,
    ) -> Result<()> {
        let c_obj = CString::new(obj)?;
        let c_decl = CString::new(declaration)?;

        let base_func: asGENFUNC_t = Some(Engine::generic_callback_thunk);

        let func_id = unsafe {
            let result = (self.as_vtable().asIScriptEngine_RegisterObjectBehaviour)(
                self.inner,
                c_obj.as_ptr(),
                behaviour.into(),
                c_decl.as_ptr(),
                &mut asGenericFunction(base_func),
                asECallConvTypes_asCALL_GENERIC,
                auxiliary.map_or_else(|| ptr::null_mut(), |aux| aux as *mut _ as *mut c_void),
                composite_offset.unwrap_or_else(|| 0),
                is_composite_indirect.unwrap_or_else(|| false),
            );

            Error::from_code(result).map(|_| result)
        }?;

        if let Some(func_obj) = self.get_function_by_id(func_id) {
            let user_data = Box::new(GenericFnUserData(func_ptr));
            func_obj.set_user_data(Box::leak(user_data));
        }

        Ok(())
    }

    // Interfaces
    pub fn register_interface(&self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterInterface)(
                self.inner,
                c_name.as_ptr(),
            ))
        }
    }

    pub fn register_interface_method(&self, intf: &str, declaration: &str) -> Result<()> {
        let c_intf = CString::new(intf)?;
        let c_decl = CString::new(declaration)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterInterfaceMethod)(
                self.inner,
                c_intf.as_ptr(),
                c_decl.as_ptr(),
            ))
        }
    }

    pub fn get_object_type_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetObjectTypeCount)(self.inner) }
    }

    pub fn get_object_type_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info =
                (self.as_vtable().asIScriptEngine_GetObjectTypeByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // String factory
    pub(crate) fn register_string_factory(
        &self,
        datatype: &str,
        factory: *mut asIStringFactory,
    ) -> Result<()> {
        let c_datatype = CString::new(datatype)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterStringFactory)(
                self.inner,
                c_datatype.as_ptr(),
                factory,
            ))
        }
    }

    pub fn get_string_factory_return_type_id(&self) -> Result<(i32, asDWORD)> {
        let mut flags: asDWORD = 0;

        unsafe {
            let type_id = (self
                .as_vtable()
                .asIScriptEngine_GetStringFactoryReturnTypeId)(
                self.inner, &mut flags
            );
            if type_id < 0 {
                Error::from_code(type_id)?;
            }
            Ok((type_id, flags))
        }
    }

    // Default array
    pub fn register_default_array_type(&self, type_name: &str) -> Result<()> {
        let c_type = CString::new(type_name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterDefaultArrayType)(
                self.inner,
                c_type.as_ptr(),
            ))
        }
    }

    pub fn get_default_array_type_id(&self) -> Result<i32> {
        unsafe {
            let type_id = (self.as_vtable().asIScriptEngine_GetDefaultArrayTypeId)(self.inner);
            if type_id < 0 {
                Error::from_code(type_id)?;
            }
            Ok(type_id)
        }
    }

    // Enums
    pub fn register_enum(&self, type_name: &str) -> Result<()> {
        let c_type = CString::new(type_name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterEnum)(
                self.inner,
                c_type.as_ptr(),
            ))
        }
    }

    pub fn register_enum_value(&self, type_name: &str, name: &str, value: i32) -> Result<()> {
        let c_type = CString::new(type_name)?;
        let c_name = CString::new(name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterEnumValue)(
                self.inner,
                c_type.as_ptr(),
                c_name.as_ptr(),
                value,
            ))
        }
    }

    pub fn get_enum_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetEnumCount)(self.inner) }
    }

    pub fn get_enum_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptEngine_GetEnumByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Funcdefs
    pub fn register_funcdef(&self, decl: &str) -> Result<()> {
        let c_decl = CString::new(decl)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterFuncdef)(
                self.inner,
                c_decl.as_ptr(),
            ))
        }
    }

    pub fn get_funcdef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetFuncdefCount)(self.inner) }
    }

    pub fn get_funcdef_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptEngine_GetFuncdefByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Typedefs
    pub fn register_typedef(&self, type_name: &str, decl: &str) -> Result<()> {
        let c_type = CString::new(type_name)?;
        let c_decl = CString::new(decl)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RegisterTypedef)(
                self.inner,
                c_type.as_ptr(),
                c_decl.as_ptr(),
            ))
        }
    }

    pub fn get_typedef_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetTypedefCount)(self.inner) }
    }

    pub fn get_typedef_by_index(&self, index: asUINT) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptEngine_GetTypedefByIndex)(self.inner, index);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Configuration groups
    pub fn begin_config_group(&self, group_name: &str) -> Result<()> {
        let c_group = CString::new(group_name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_BeginConfigGroup)(
                self.inner,
                c_group.as_ptr(),
            ))
        }
    }

    pub fn end_config_group(&self) -> Result<()> {
        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_EndConfigGroup)(
                self.inner,
            ))
        }
    }

    pub fn remove_config_group(&self, group_name: &str) -> Result<()> {
        let c_group = CString::new(group_name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RemoveConfigGroup)(
                self.inner,
                c_group.as_ptr(),
            ))
        }
    }

    // Access control
    pub fn set_default_access_mask(&self, default_mask: asDWORD) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptEngine_SetDefaultAccessMask)(self.inner, default_mask) }
    }

    // Namespaces
    pub fn set_default_namespace(&self, name_space: &str) -> Result<()> {
        let c_namespace = CString::new(name_space)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_SetDefaultNamespace)(
                self.inner,
                c_namespace.as_ptr(),
            ))
        }
    }

    pub fn get_default_namespace(&self) -> Option<&str> {
        unsafe {
            let namespace = (self.as_vtable().asIScriptEngine_GetDefaultNamespace)(self.inner);
            if namespace.is_null() {
                None
            } else {
                CStr::from_ptr(namespace).to_str().ok()
            }
        }
    }

    // Modules
    pub fn get_module(&self, name: &str, flag: GetModuleFlags) -> Result<Module> {
        let c_name = CString::new(name)?;

        unsafe {
            let module = (self.as_vtable().asIScriptEngine_GetModule)(
                self.inner,
                c_name.as_ptr(),
                flag.into(),
            );
            if module.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Module::from_raw(module))
            }
        }
    }

    pub fn discard_module(&self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_DiscardModule)(
                self.inner,
                c_name.as_ptr(),
            ))
        }
    }

    pub fn get_module_count(&self) -> asUINT {
        unsafe { (self.as_vtable().asIScriptEngine_GetModuleCount)(self.inner) }
    }

    pub fn get_module_by_index(&self, index: asUINT) -> Option<Module> {
        unsafe {
            let module = (self.as_vtable().asIScriptEngine_GetModuleByIndex)(self.inner, index);
            if module.is_null() {
                None
            } else {
                Some(Module::from_raw(module))
            }
        }
    }

    // Functions
    pub fn get_last_function_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptEngine_GetLastFunctionId)(self.inner) }
    }

    pub fn get_function_by_id(&self, func_id: i32) -> Option<Function> {
        unsafe {
            let func_ptr = (self.as_vtable().asIScriptEngine_GetFunctionById)(self.inner, func_id);
            if func_ptr.is_null() {
                None
            } else {
                Some(Function::from_raw(func_ptr))
            }
        }
    }

    // Type information
    pub fn get_type_id_by_decl(&self, decl: &str) -> Result<i32> {
        let c_decl = CString::new(decl)?;

        unsafe {
            let type_id =
                (self.as_vtable().asIScriptEngine_GetTypeIdByDecl)(self.inner, c_decl.as_ptr());
            if type_id < 0 {
                Error::from_code(type_id)?;
            }
            Ok(type_id)
        }
    }

    pub fn get_type_declaration(&self, type_id: i32, include_namespace: bool) -> Option<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptEngine_GetTypeDeclaration)(
                self.inner,
                type_id,
                include_namespace,
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    pub fn get_size_of_primitive_type(&self, type_id: i32) -> Result<i32> {
        unsafe {
            let size =
                (self.as_vtable().asIScriptEngine_GetSizeOfPrimitiveType)(self.inner, type_id);
            if size < 0 {
                Error::from_code(size)?;
            }
            Ok(size)
        }
    }

    pub fn get_type_info_by_id(&self, type_id: i32) -> Option<TypeInfo> {
        unsafe {
            let type_info = (self.as_vtable().asIScriptEngine_GetTypeInfoById)(self.inner, type_id);
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    pub fn get_type_info_by_name(&self, name: &str) -> Option<TypeInfo> {
        let c_name = CString::new(name).ok()?;

        unsafe {
            let type_info =
                (self.as_vtable().asIScriptEngine_GetTypeInfoByName)(self.inner, c_name.as_ptr());
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
            let type_info =
                (self.as_vtable().asIScriptEngine_GetTypeInfoByDecl)(self.inner, c_decl.as_ptr());
            if type_info.is_null() {
                None
            } else {
                Some(TypeInfo::from_raw(type_info))
            }
        }
    }

    // Context creation
    pub fn create_context(&self) -> Result<Context> {
        unsafe {
            let ctx = (self.as_vtable().asIScriptEngine_CreateContext)(self.inner);
            if ctx.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Context::from_raw(ctx))
            }
        }
    }

    // Script objects
    pub fn create_script_object(&self, type_info: &TypeInfo) -> Result<Ptr<c_void>> {
        unsafe {
            let obj = (self.as_vtable().asIScriptEngine_CreateScriptObject)(
                self.inner,
                type_info.as_ptr(),
            );
            if obj.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Ptr::<c_void>::from_raw(obj))
            }
        }
    }

    pub fn create_script_object_copy<T>(
        &self,
        obj: &mut T,
        type_info: &TypeInfo,
    ) -> Option<Ptr<T>> {
        unsafe {
            let new_obj = (self.as_vtable().asIScriptEngine_CreateScriptObjectCopy)(
                self.inner,
                obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            );
            if new_obj.is_null() {
                None
            } else {
                Some(Ptr::<T>::from_raw(new_obj))
            }
        }
    }

    pub fn create_uninitialized_script_object(&self, type_info: &TypeInfo) -> Option<Ptr<c_void>> {
        unsafe {
            let obj = (self
                .as_vtable()
                .asIScriptEngine_CreateUninitializedScriptObject)(
                self.inner, type_info.as_ptr()
            );
            if obj.is_null() {
                None
            } else {
                Some(Ptr::<c_void>::from_raw(obj))
            }
        }
    }

    pub fn create_delegate<T>(&self, func: &Function, obj: &mut T) -> Option<Function> {
        unsafe {
            let delegate = (self.as_vtable().asIScriptEngine_CreateDelegate)(
                self.inner,
                func.as_raw(),
                obj as *mut _ as *mut c_void,
            );
            if delegate.is_null() {
                None
            } else {
                Some(Function::from_raw(delegate))
            }
        }
    }

    pub fn assign_script_object<T>(
        &self,
        dst_obj: &mut T,
        src_obj: &mut T,
        type_info: &TypeInfo,
    ) -> Result<()> {
        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_AssignScriptObject)(
                self.inner,
                dst_obj as *mut _ as *mut c_void,
                src_obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            ))
        }
    }

    pub fn release_script_object<T>(&self, obj: &mut T, type_info: &TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ReleaseScriptObject)(
                self.inner,
                obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            );
        }
    }

    pub fn add_ref_script_object<T>(&self, obj: &mut T, type_info: &TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_AddRefScriptObject)(
                self.inner,
                obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            );
        }
    }

    pub fn ref_cast_object<T, U>(
        &self,
        obj: &mut T,
        from_type: &mut TypeInfo,
        to_type: &mut TypeInfo,
        use_only_implicit_cast: bool,
    ) -> Result<Option<Ptr<U>>> {
        let mut new_ptr: *mut c_void = ptr::null_mut();

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_RefCastObject)(
                self.inner,
                obj as *mut _ as *mut c_void,
                from_type.as_ptr(),
                to_type.as_ptr(),
                &mut new_ptr,
                use_only_implicit_cast,
            ))?;

            if new_ptr.is_null() {
                Ok(None)
            } else {
                Ok(Some(Ptr::<U>::from_raw(new_ptr)))
            }
        }
    }

    pub fn get_weak_ref_flag_of_script_object<T>(
        &self,
        obj: &mut T,
        type_info: &TypeInfo,
    ) -> Option<LockableSharedBool> {
        unsafe {
            let flag = (self
                .as_vtable()
                .asIScriptEngine_GetWeakRefFlagOfScriptObject)(
                self.inner,
                obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            );
            if flag.is_null() {
                None
            } else {
                Some(LockableSharedBool::from_raw(flag))
            }
        }
    }

    // Context management
    pub fn request_context(&self) -> Option<Context> {
        unsafe {
            let ctx = (self.as_vtable().asIScriptEngine_RequestContext)(self.inner);
            if ctx.is_null() {
                None
            } else {
                Some(Context::from_raw(ctx))
            }
        }
    }

    pub fn return_context(&self, ctx: Context) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ReturnContext)(self.inner, ctx.as_ptr());
        }
    }

    pub fn set_context_callbacks<T>(
        &mut self,
        request_ctx: RequestContextCallbackFn,
        return_ctx: ReturnContextCallbackFn,
        param: &mut T,
    ) -> Result<()> {
        CallbackManager::set_request_context_callback(Some(request_ctx))?;
        CallbackManager::set_return_context_callback(Some(return_ctx))?;

        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_SetContextCallbacks)(
                self.inner,
                Some(CallbackManager::cvoid_request_context_callback),
                Some(CallbackManager::cvoid_return_context_callback),
                param as *mut _ as *mut c_void,
            ))
        }
    }

    // Parsing
    pub fn parse_token(&self, string: &str) -> (TokenClass, usize) {
        let c_string = string.as_bytes();
        let mut token_length: asUINT = 0;

        unsafe {
            let token_class = (self.as_vtable().asIScriptEngine_ParseToken)(
                self.inner,
                c_string.as_ptr() as *const c_char,
                c_string.len(),
                &mut token_length,
            );
            (token_class.into(), token_length as usize)
        }
    }

    // Garbage collection
    pub fn garbage_collect(&self, flags: asDWORD, num_iterations: asUINT) -> Result<()> {
        unsafe {
            Error::from_code((self.as_vtable().asIScriptEngine_GarbageCollect)(
                self.inner,
                flags,
                num_iterations,
            ))
        }
    }

    pub fn get_gc_statistics(&self) -> GCStatistics {
        let mut current_size: asUINT = 0;
        let mut total_destroyed: asUINT = 0;
        let mut total_detected: asUINT = 0;
        let mut new_objects: asUINT = 0;
        let mut total_new_destroyed: asUINT = 0;

        unsafe {
            (self.as_vtable().asIScriptEngine_GetGCStatistics)(
                self.inner,
                &mut current_size,
                &mut total_destroyed,
                &mut total_detected,
                &mut new_objects,
                &mut total_new_destroyed,
            );
        }

        GCStatistics {
            current_size,
            total_destroyed,
            total_detected,
            new_objects,
            total_new_destroyed,
        }
    }

    pub fn notify_garbage_collector_of_new_object<T>(
        &self,
        obj: &mut T,
        type_info: &mut TypeInfo,
    ) -> Result<()> {
        unsafe {
            Error::from_code((self
                .as_vtable()
                .asIScriptEngine_NotifyGarbageCollectorOfNewObject)(
                self.inner,
                obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            ))
        }
    }

    pub fn get_object_in_gc(&self, idx: asUINT) -> Option<GCObjectInfo> {
        let mut seq_nbr: asUINT = 0;
        let mut obj: *mut c_void = ptr::null_mut();
        let mut type_info: *mut asITypeInfo = ptr::null_mut();

        unsafe {
            let result = (self.as_vtable().asIScriptEngine_GetObjectInGC)(
                self.inner,
                idx,
                &mut seq_nbr,
                &mut obj,
                &mut type_info,
            );

            if result < 0 || obj.is_null() {
                None
            } else {
                Some(GCObjectInfo {
                    seq_nbr,
                    obj: Ptr::<c_void>::from_raw(obj),
                    type_info: if type_info.is_null() {
                        None
                    } else {
                        Some(TypeInfo::from_raw(type_info))
                    },
                })
            }
        }
    }

    pub fn gc_enum_callback<T>(&self, reference: &mut T) {
        unsafe {
            (self.as_vtable().asIScriptEngine_GCEnumCallback)(
                self.inner,
                reference as *mut _ as *mut c_void,
            );
        }
    }

    pub fn forward_gc_enum_references<T>(&self, ref_obj: &mut T, type_info: &mut TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ForwardGCEnumReferences)(
                self.inner,
                ref_obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            );
        }
    }

    pub fn forward_gc_release_references<T>(&self, ref_obj: &mut T, type_info: &mut TypeInfo) {
        unsafe {
            (self.as_vtable().asIScriptEngine_ForwardGCReleaseReferences)(
                self.inner,
                ref_obj as *mut _ as *mut c_void,
                type_info.as_ptr(),
            );
        }
    }

    pub fn set_circular_ref_detected_callback(
        &mut self,
        callback: CircularRefCallbackFn,
    ) -> Result<()> {
        CallbackManager::set_circular_ref_callback(Some(callback))?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetCircularRefDetectedCallback)(
                self.inner,
                Some(CallbackManager::cvoid_circular_ref_callback),
                ptr::null_mut(),
            );
        }

        Ok(())
    }

    // User data
    pub fn set_user_data<T: UserData>(&self, data: &mut T) -> Option<Ptr<T>> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptEngine_SetUserData)(
                self.inner,
                data as *mut _ as *mut c_void,
                T::TypeId as asPWORD,
            );
            if ptr.is_null() {
                None
            } else {
                Some(Ptr::<T>::from_raw(ptr))
            }
        }
    }

    pub fn get_user_data<T: UserData>(&self) -> Result<Ptr<T>> {
        unsafe {
            let ptr =
                (self.as_vtable().asIScriptEngine_GetUserData)(self.inner, T::TypeId as asPWORD);
            if ptr.is_null() {
                Err(Error::NullPointer)
            } else {
                Ok(Ptr::<T>::from_raw(ptr))
            }
        }
    }

    pub fn set_engine_user_data_cleanup_callback(
        &mut self,
        callback: CleanEngineUserDataCallbackFn,
        type_id: asPWORD,
    ) -> Result<()> {
        CallbackManager::add_engine_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetEngineUserDataCleanupCallback)(
                self.inner,
                Some(CallbackManager::cvoid_engine_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    pub fn set_module_user_data_cleanup_callback(
        &mut self,
        callback: CleanModuleUserDataCallbackFn,
        type_id: asPWORD,
    ) -> Result<()> {
        CallbackManager::add_module_user_data_cleanup_callback(type_id, callback)?;
        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetModuleUserDataCleanupCallback)(
                self.inner,
                Some(CallbackManager::cvoid_module_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    pub fn set_context_user_data_cleanup_callback(
        &mut self,
        callback: CleanContextUserDataCallbackFn,
        type_id: asPWORD,
    ) -> Result<()> {
        CallbackManager::add_context_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetContextUserDataCleanupCallback)(
                self.inner,
                Some(CallbackManager::cvoid_context_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    pub fn set_function_user_data_cleanup_callback(
        &mut self,
        callback: CleanFunctionUserDataCallbackFn,
        type_id: asPWORD,
    ) -> Result<()> {
        CallbackManager::add_function_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetFunctionUserDataCleanupCallback)(
                self.inner,
                Some(CallbackManager::cvoid_function_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    pub fn set_type_info_user_data_cleanup_callback(
        &mut self,
        callback: CleanTypeInfoCallbackFn,
        type_id: asPWORD,
    ) -> Result<()> {
        CallbackManager::add_type_info_user_data_cleanup_callback(type_id, callback)?;

        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetTypeInfoUserDataCleanupCallback)(
                self.inner,
                Some(CallbackManager::cvoid_type_info_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    pub fn set_script_object_user_data_cleanup_callback(
        &mut self,
        callback: CleanScriptObjectCallbackFn,
        type_id: asPWORD,
    ) -> Result<()> {
        CallbackManager::add_script_object_user_data_cleanup_callback(type_id, callback)?;
        unsafe {
            (self
                .as_vtable()
                .asIScriptEngine_SetScriptObjectUserDataCleanupCallback)(
                self.inner,
                Some(CallbackManager::cvoid_script_object_user_data_cleanup_callback),
                type_id,
            );
        }

        Ok(())
    }

    // Translate app exception callback

    pub fn set_translate_app_exception_callback(
        &mut self,
        callback: TranslateAppExceptionCallbackFn,
    ) -> Result<()> {
        CallbackManager::set_translate_exception_callback(Some(callback))?;

        let conv: u32 = CallingConvention::Cdecl.into();
        unsafe {
            Error::from_code((self
                .as_vtable()
                .asIScriptEngine_SetTranslateAppExceptionCallback)(
                self.inner,
                asScriptContextFunction(Some(CallbackManager::cvoid_translate_exception_callback)),
                ptr::null_mut(),
                conv as i32,
            ))
        }
    }

    pub fn with_default_modules(&self) -> Result<()> {
        #[cfg(feature = "string")]
        with_string_module(self)?;

        Ok(())
    }

    fn as_vtable(&self) -> &asIScriptEngine__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        match self.is_root {
            true => {
                self.shutdown_and_release()
                    .expect("Failed to shutdown engine");
            }
            false => {
                self.release().expect("Failed to release engine");
            }
        };
    }
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Engine::from_raw(self.inner)
    }
}

unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

struct GenericFnUserData(pub GenericFn);

impl GenericFnUserData {
    pub fn call(&self, ctx: &ScriptGeneric) {
        (self.0)(ctx)
    }
}

unsafe impl Send for GenericFnUserData {}
unsafe impl Sync for GenericFnUserData {}

impl UserData for GenericFnUserData {
    const TypeId: usize = 0x129032719; // Must be unique!
}

#[derive(Debug)]
pub struct GlobalPropertyInfo {
    pub name: Option<String>,
    pub name_space: Option<String>,
    pub type_id: i32,
    pub is_const: bool,
    pub config_group: Option<String>,
    pub pointer: Ptr<std::os::raw::c_void>,
    pub access_mask: asDWORD,
}

#[derive(Debug)]
pub struct GCStatistics {
    pub current_size: asUINT,
    pub total_destroyed: asUINT,
    pub total_detected: asUINT,
    pub new_objects: asUINT,
    pub total_new_destroyed: asUINT,
}

#[derive(Debug)]
pub struct GCObjectInfo {
    pub seq_nbr: asUINT,
    pub obj: Ptr<std::os::raw::c_void>,
    pub type_info: Option<TypeInfo>,
}
