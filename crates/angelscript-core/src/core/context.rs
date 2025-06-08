use crate::types::callbacks::{ExceptionCallbackFn, LineCallbackFn};
use angelscript_sys::{
    asBYTE, asDWORD, asETypeModifiers, asETypeModifiers_asTM_NONE, asFUNCTION_t, asFunction,
    asIScriptContext, asIScriptContext__bindgen_vtable, asIScriptEngine, asIScriptFunction,
    asITypeInfo, asQWORD, asScriptContextFunction, asUINT, asWORD,
};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::ptr::NonNull;
use crate::core::engine::Engine;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::core::typeinfo::TypeInfo;
use crate::internal::callback_manager::CallbackManager;
use crate::types::enums::{CallingConvention, ContextState, TypeModifiers};
use crate::types::script_data::ScriptData;
use crate::types::script_memory::ScriptMemoryLocation;
use crate::types::user_data::UserData;

type InternalCallback = Option<unsafe extern "C" fn(*mut asIScriptContext, *const c_void)>;

#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    context: *mut asIScriptContext,
}

impl Context {
    pub(crate) fn from_raw(context: *mut asIScriptContext) -> Self {
        let wrapper = Context { context };
        wrapper
            .add_ref()
            .expect("Failed to add reference to context");
        wrapper
    }

    // Reference counting (matches vtable order)
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_AddRef)(self.context)) }
    }

    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_Release)(self.context)) }
    }

    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptContext_GetEngine)(self.context);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    // Execution control
    pub fn prepare(&self, func: &Function) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_Prepare)(
                self.context,
                func.as_raw(),
            ))
        }
    }

    pub fn unprepare(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_Unprepare)(self.context))
        }
    }

    pub fn execute(&self) -> ScriptResult<ContextState> {
        unsafe {
            let result = (self.as_vtable().asIScriptContext_Execute)(self.context);
            ScriptError::from_code(result)?;
            Ok(ContextState::from(result as u32))
        }
    }

    pub fn abort(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_Abort)(self.context)) }
    }

    pub fn suspend(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_Suspend)(self.context)) }
    }

    pub fn get_state(&self) -> ContextState {
        unsafe { ContextState::from((self.as_vtable().asIScriptContext_GetState)(self.context)) }
    }

    // State management
    pub fn push_state(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_PushState)(self.context))
        }
    }

    pub fn pop_state(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_PopState)(self.context))
        }
    }

    pub fn is_nested(&self) -> (bool, u32) {
        let mut nest_count: u32 = 0;
        unsafe {
            let is_nested =
                (self.as_vtable().asIScriptContext_IsNested)(self.context, &mut nest_count);
            (is_nested, nest_count)
        }
    }

    // Object context
    pub fn set_object<T: ScriptData>(&self, obj: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetObject)(
                self.context,
                obj.to_script_ptr(),
            ))
        }
    }

    // Arguments (in vtable order)
    pub fn set_arg_byte(&self, arg: asUINT, value: asBYTE) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgByte)(
                self.context,
                arg,
                value,
            ))
        }
    }

    pub fn set_arg_word(&self, arg: asUINT, value: asWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgWord)(
                self.context,
                arg,
                value,
            ))
        }
    }

    pub fn set_arg_dword(&self, arg: asUINT, value: asDWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgDWord)(
                self.context,
                arg,
                value,
            ))
        }
    }

    pub fn set_arg_qword(&self, arg: asUINT, value: asQWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgQWord)(
                self.context,
                arg,
                value,
            ))
        }
    }

    pub fn set_arg_float(&self, arg: asUINT, value: f32) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgFloat)(
                self.context,
                arg,
                value,
            ))
        }
    }

    pub fn set_arg_double(&self, arg: asUINT, value: f64) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgDouble)(
                self.context,
                arg,
                value,
            ))
        }
    }

    pub fn set_arg_address<T: ScriptData>(&self, arg: asUINT, addr: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgAddress)(
                self.context,
                arg,
                addr.to_script_ptr(),
            ))
        }
    }

    pub fn set_arg_object<T: ScriptData>(&self, arg: asUINT, obj: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgObject)(
                self.context,
                arg,
                obj.to_script_ptr(),
            ))
        }
    }

    pub fn set_arg_var_type<T: ScriptData>(
        &self,
        arg: asUINT,
        ptr: &mut T,
        type_id: i32,
    ) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgVarType)(
                self.context,
                arg,
                ptr.to_script_ptr(),
                type_id,
            ))
        }
    }

    pub fn get_address_of_arg<T: ScriptData>(&self, arg: asUINT) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetAddressOfArg)(self.context, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // Return values (in vtable order)
    pub fn get_return_byte(&self) -> asBYTE {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnByte)(self.context) }
    }

    pub fn get_return_word(&self) -> asWORD {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnWord)(self.context) }
    }

    pub fn get_return_dword(&self) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnDWord)(self.context) }
    }

    pub fn get_return_qword(&self) -> asQWORD {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnQWord)(self.context) }
    }

    pub fn get_return_float(&self) -> f32 {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnFloat)(self.context) }
    }

    pub fn get_return_double(&self) -> f64 {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnDouble)(self.context) }
    }

    pub fn get_return_address<T: ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetReturnAddress)(self.context);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    pub fn get_return_object<T: ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetReturnObject)(self.context);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    pub fn get_address_of_return_value<T: ScriptData>(&self) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetAddressOfReturnValue)(self.context);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // Exception handling
    pub fn set_exception(&self, string: &str, allow_catch: bool) -> ScriptResult<()> {
        let c_string = CString::new(string)?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetException)(
                self.context,
                c_string.as_ptr(),
                allow_catch,
            ))
        }
    }

    pub fn get_exception_line_number(&self) -> (i32, Option<i32>, Option<&str>) {
        let mut column: i32 = 0;
        let mut section_name: *const c_char = ptr::null();

        unsafe {
            let line = (self.as_vtable().asIScriptContext_GetExceptionLineNumber)(
                self.context,
                &mut column,
                &mut section_name,
            );

            let section = if section_name.is_null() {
                None
            } else {
                CStr::from_ptr(section_name).to_str().ok()
            };

            (line, Some(column), section)
        }
    }

    pub fn get_exception_function(&self) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptContext_GetExceptionFunction)(self.context);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_exception_string(&self) -> Option<&str> {
        unsafe {
            let string = (self.as_vtable().asIScriptContext_GetExceptionString)(self.context);
            if string.is_null() {
                None
            } else {
                CStr::from_ptr(string).to_str().ok()
            }
        }
    }

    pub fn will_exception_be_caught(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptContext_WillExceptionBeCaught)(self.context) }
    }

    pub fn set_exception_callback(&self, callback: ExceptionCallbackFn) -> ScriptResult<()> {
        CallbackManager::set_exception_callback(Some(callback))?;

        let base_func: InternalCallback = Some(CallbackManager::cvoid_exception_callback);
        let c_func = unsafe { std::mem::transmute::<InternalCallback, asFUNCTION_t>(base_func) };

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetExceptionCallback)(
                self.context,
                asFunction(c_func),
                std::ptr::null_mut(),
                CallingConvention::Cdecl as i32,
            ))
        }
    }

    pub fn clear_exception_callback(&self) -> ScriptResult<()> {
        CallbackManager::set_exception_callback(None)?;
        unsafe {
            (self.as_vtable().asIScriptContext_ClearExceptionCallback)(self.context);
        }
        Ok(())
    }

    pub fn set_line_callback<T: ScriptData>(
        &mut self,
        callback: LineCallbackFn,
        param: &mut T,
    ) -> ScriptResult<()> {
        CallbackManager::set_line_callback(Some(callback))?;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetLineCallback)(
                self.context,
                asScriptContextFunction(Some(CallbackManager::cvoid_line_callback)),
                param.to_script_ptr(),
                CallingConvention::Cdecl as i32,
            ))
        }
    }

    pub fn clear_line_callback(&mut self) -> ScriptResult<()> {
        CallbackManager::set_line_callback(None)?;
        unsafe {
            (self.as_vtable().asIScriptContext_ClearLineCallback)(self.context);
        }
        Ok(())
    }

    // Call stack inspection
    pub fn get_callstack_size(&self) -> u32 {
        unsafe { (self.as_vtable().asIScriptContext_GetCallstackSize)(self.context) }
    }

    pub fn get_function(&self, stack_level: u32) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptContext_GetFunction)(self.context, stack_level);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_line_number(&self, stack_level: u32) -> (i32, Option<i32>, Option<&str>) {
        let mut column: i32 = 0;
        let mut section_name: *const c_char = ptr::null();

        unsafe {
            let line = (self.as_vtable().asIScriptContext_GetLineNumber)(
                self.context,
                stack_level,
                &mut column,
                &mut section_name,
            );

            let section = if section_name.is_null() {
                None
            } else {
                CStr::from_ptr(section_name).to_str().ok()
            };

            (line, Some(column), section)
        }
    }

    // Variable inspection
    pub fn get_var_count(&self, stack_level: u32) -> i32 {
        unsafe { (self.as_vtable().asIScriptContext_GetVarCount)(self.context, stack_level) }
    }

    pub fn get_var(&self, var_index: u32, stack_level: u32) -> ScriptResult<VariableInfo> {
        let mut name: *const c_char = ptr::null();
        let mut type_id: i32 = 0;
        let mut type_modifiers: asETypeModifiers = asETypeModifiers_asTM_NONE;
        let mut is_var_on_heap: bool = false;
        let mut stack_offset: i32 = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_GetVar)(
                self.context,
                var_index,
                stack_level,
                &mut name,
                &mut type_id,
                &mut type_modifiers,
                &mut is_var_on_heap,
                &mut stack_offset,
            ))?;

            let name_str = if name.is_null() {
                None
            } else {
                CStr::from_ptr(name).to_str().ok()
            };

            Ok(VariableInfo {
                name: name_str.map(|s| s.to_string()),
                type_id,
                type_modifiers: TypeModifiers::from_bits_truncate(type_modifiers),
                is_var_on_heap,
                stack_offset,
            })
        }
    }

    pub fn get_var_declaration(
        &self,
        var_index: u32,
        stack_level: u32,
        include_namespace: bool,
    ) -> Option<&str> {
        unsafe {
            let decl = (self.as_vtable().asIScriptContext_GetVarDeclaration)(
                self.context,
                var_index,
                stack_level,
                include_namespace,
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    pub fn get_address_of_var<T: ScriptData>(
        &self,
        var_index: u32,
        stack_level: u32,
        dont_dereference: bool,
        return_address_of_uninitialized_objects: bool,
    ) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetAddressOfVar)(
                self.context,
                var_index,
                stack_level,
                dont_dereference,
                return_address_of_uninitialized_objects,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    pub fn is_var_in_scope(&self, var_index: u32, stack_level: u32) -> bool {
        unsafe {
            (self.as_vtable().asIScriptContext_IsVarInScope)(self.context, var_index, stack_level)
        }
    }

    // This pointer
    pub fn get_this_type_id(&self, stack_level: u32) -> i32 {
        unsafe { (self.as_vtable().asIScriptContext_GetThisTypeId)(self.context, stack_level) }
    }

    pub fn get_this_pointer<T: ScriptData>(&self, stack_level: u32) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetThisPointer)(self.context, stack_level);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // System function
    pub fn get_system_function(&self) -> Option<Function> {
        unsafe {
            let func = (self.as_vtable().asIScriptContext_GetSystemFunction)(self.context);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // User data
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_SetUserData)(
                self.context,
                data.to_script_ptr(),
                T::TYPE_ID,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    pub fn get_user_data<T: UserData + ScriptData>(&self) -> ScriptResult<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetUserData)(self.context, T::TYPE_ID);
            if ptr.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // Serialization
    pub fn start_deserialization(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_StartDeserialization)(
                self.context,
            ))
        }
    }

    pub fn finish_deserialization(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_FinishDeserialization)(
                self.context,
            ))
        }
    }

    pub fn push_function<T: ScriptData>(
        &self,
        func: &Function,
        object: Option<&mut T>,
    ) -> ScriptResult<()> {
        unsafe {
            let obj_ptr = match object {
                Some(obj) => obj.to_script_ptr(),
                None => ptr::null_mut(),
            };
            ScriptError::from_code((self.as_vtable().asIScriptContext_PushFunction)(
                self.context,
                func.as_raw(),
                obj_ptr,
            ))
        }
    }

    // Advanced debugging - state registers
    pub fn get_state_registers(&self, stack_level: u32) -> ScriptResult<StateRegisters> {
        let mut calling_system_function: *mut asIScriptFunction = ptr::null_mut();
        let mut initial_function: *mut asIScriptFunction = ptr::null_mut();
        let mut orig_stack_pointer: asDWORD = 0;
        let mut arguments_size: asDWORD = 0;
        let mut value_register: asQWORD = 0;
        let mut object_register: *mut c_void = ptr::null_mut();
        let mut object_type_register: *mut asITypeInfo = ptr::null_mut();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_GetStateRegisters)(
                self.context,
                stack_level,
                &mut calling_system_function,
                &mut initial_function,
                &mut orig_stack_pointer,
                &mut arguments_size,
                &mut value_register,
                &mut object_register,
                &mut object_type_register,
            ))?;

            Ok(StateRegisters {
                calling_system_function: if calling_system_function.is_null() {
                    None
                } else {
                    Some(Function::from_raw(calling_system_function))
                },
                initial_function: if initial_function.is_null() {
                    None
                } else {
                    Some(Function::from_raw(initial_function))
                },
                orig_stack_pointer,
                arguments_size,
                value_register,
                object_register: if object_register.is_null() {
                    None
                } else {
                    Some(ScriptMemoryLocation::from_mut(object_register))
                },
                object_type_register: if object_type_register.is_null() {
                    None
                } else {
                    Some(TypeInfo::from_raw(object_type_register))
                },
            })
        }
    }

    pub fn get_call_state_registers(&self, stack_level: u32) -> ScriptResult<CallStateRegisters> {
        let mut stack_frame_pointer: asDWORD = 0;
        let mut current_function: *mut asIScriptFunction = ptr::null_mut();
        let mut program_pointer: asDWORD = 0;
        let mut stack_pointer: asDWORD = 0;
        let mut stack_index: asDWORD = 0;

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_GetCallStateRegisters)(
                self.context,
                stack_level,
                &mut stack_frame_pointer,
                &mut current_function,
                &mut program_pointer,
                &mut stack_pointer,
                &mut stack_index,
            ))?;

            Ok(CallStateRegisters {
                stack_frame_pointer,
                current_function: if current_function.is_null() {
                    None
                } else {
                    Some(Function::from_raw(current_function))
                },
                program_pointer,
                stack_pointer,
                stack_index,
            })
        }
    }

    pub fn set_state_registers(
        &self,
        stack_level: u32,
        calling_system_function: Option<&Function>,
        initial_function: Option<&Function>,
        orig_stack_pointer: asDWORD,
        arguments_size: asDWORD,
        value_register: asQWORD,
        object_register: Option<Box<dyn ScriptData>>,
        object_type_register: Option<&TypeInfo>,
    ) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetStateRegisters)(
                self.context,
                stack_level,
                calling_system_function.map_or_else(ptr::null_mut, |f| f.as_raw()),
                initial_function.map_or_else(ptr::null_mut, |f| f.as_raw()),
                orig_stack_pointer,
                arguments_size,
                value_register,
                object_register.map_or(ptr::null_mut(), |mut p| p.to_script_ptr()),
                object_type_register.map_or(ptr::null_mut(), |t| t.as_ptr()),
            ))
        }
    }

    pub fn set_call_state_registers(
        &self,
        stack_level: u32,
        stack_frame_pointer: asDWORD,
        current_function: Option<&Function>,
        program_pointer: asDWORD,
        stack_pointer: asDWORD,
        stack_index: asDWORD,
    ) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetCallStateRegisters)(
                self.context,
                stack_level,
                stack_frame_pointer,
                current_function.map_or_else(ptr::null_mut, |f| f.as_raw()),
                program_pointer,
                stack_pointer,
                stack_index,
            ))
        }
    }

    // Stack argument inspection
    pub fn get_args_on_stack_count(&self, stack_level: u32) -> ScriptResult<i32> {
        unsafe {
            let count =
                (self.as_vtable().asIScriptContext_GetArgsOnStackCount)(self.context, stack_level);
            if count < 0 {
                ScriptError::from_code(count)?;
            }
            Ok(count)
        }
    }

    pub fn get_arg_on_stack<T: ScriptData>(
        &self,
        stack_level: u32,
        arg: u32,
    ) -> ScriptResult<StackArgument<T>> {
        let mut type_id: i32 = 0;
        let mut flags: asUINT = 0;
        let mut address: *mut c_void = ptr::null_mut();

        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_GetArgOnStack)(
                self.context,
                stack_level,
                arg,
                &mut type_id,
                &mut flags,
                &mut address,
            ))?;

            Ok(StackArgument {
                type_id,
                flags,
                address: if address.is_null() {
                    None
                } else {
                    Some(ScriptData::from_script_ptr(address))
                },
            })
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut asIScriptContext {
        self.context
    }

    fn as_vtable(&self) -> &asIScriptContext__bindgen_vtable {
        unsafe { &*(*self.context).vtable_ }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        self.release().expect("Failed to release context");
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: Option<String>,
    pub type_id: i32,
    pub type_modifiers: TypeModifiers,
    pub is_var_on_heap: bool,
    pub stack_offset: i32,
}

#[derive(Debug)]
pub struct StateRegisters {
    pub calling_system_function: Option<Function>,
    pub initial_function: Option<Function>,
    pub orig_stack_pointer: asDWORD,
    pub arguments_size: asDWORD,
    pub value_register: asQWORD,
    pub object_register: Option<ScriptMemoryLocation>,
    pub object_type_register: Option<TypeInfo>,
}

#[derive(Debug)]
pub struct CallStateRegisters {
    pub stack_frame_pointer: asDWORD,
    pub current_function: Option<Function>,
    pub program_pointer: asDWORD,
    pub stack_pointer: asDWORD,
    pub stack_index: asDWORD,
}

#[derive(Debug)]
pub struct StackArgument<T> {
    pub type_id: i32,
    pub flags: asUINT,
    pub address: Option<T>,
}
