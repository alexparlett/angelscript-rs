use crate::enums::*;
use crate::error::{Error, Result};
use crate::ffi::{
    asContext_Abort, asContext_AddRef, asContext_Execute, asContext_GetCallstackSize,
    asContext_GetEngine, asContext_GetExceptionFunction, asContext_GetExceptionLineNumber,
    asContext_GetExceptionString, asContext_GetFunction, asContext_GetLineNumber,
    asContext_GetReturnByte, asContext_GetReturnDWord, asContext_GetReturnDouble,
    asContext_GetReturnFloat, asContext_GetReturnObject, asContext_GetReturnQWord,
    asContext_GetReturnWord, asContext_GetState, asContext_GetSystemFunction,
    asContext_GetThisPointer, asContext_GetThisTypeId, asContext_GetUserData,
    asContext_GetVarCount, asContext_GetVarDeclaration, asContext_IsNested, asContext_IsVarInScope,
    asContext_PopState, asContext_Prepare, asContext_PushState, asContext_Release,
    asContext_SetArgByte, asContext_SetArgDWord, asContext_SetArgDouble, asContext_SetArgFloat,
    asContext_SetArgObject, asContext_SetArgQWord, asContext_SetArgWord, asContext_SetException,
    asContext_SetObject, asContext_SetUserData, asContext_Suspend, asContext_Unprepare,
    asIScriptContext,
};
use crate::function::Function;
use crate::types::*;
use crate::utils::{as_bool, from_as_bool, FromCVoidPtr};
use crate::Engine;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

pub struct Context {
    context: *mut asIScriptContext,
}

impl Context {
    pub(crate) fn from_raw(context: *mut asIScriptContext) -> Self {
        Context { context }
    }

    pub fn get_engine(&self) -> Engine {
        unsafe { Engine::from_raw(asContext_GetEngine(self.context)) }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_AddRef(self.context)) }
    }

    pub fn release(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_Release(self.context)) }
    }

    // Execution
    pub fn get_state(&self) -> ContextState {
        unsafe { asContext_GetState(self.context) }
    }

    pub fn prepare(&self, func: &Function) -> Result<()> {
        unsafe { Error::from_code(asContext_Prepare(self.context, func.as_ptr())) }
    }

    pub fn unprepare(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_Unprepare(self.context)) }
    }

    pub fn execute(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_Execute(self.context)) }
    }

    pub fn abort(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_Abort(self.context)) }
    }

    pub fn suspend(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_Suspend(self.context)) }
    }

    // State management
    pub fn push_state(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_PushState(self.context)) }
    }

    pub fn pop_state(&self) -> Result<()> {
        unsafe { Error::from_code(asContext_PopState(self.context)) }
    }

    pub fn is_nested(&self) -> (bool, u32) {
        let mut nest_count: u32 = 0;
        unsafe {
            let is_nested = asContext_IsNested(self.context, &mut nest_count);
            (from_as_bool(is_nested), nest_count)
        }
    }

    // Object pointer for calling class methods
    pub fn set_object<T>(&self, obj: &mut T) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetObject(
                self.context,
                obj as *mut _ as *mut c_void,
            ))
        }
    }

    // Arguments
    pub fn set_arg_u8(&self, arg: u32, value: u8) -> Result<()> {
        unsafe { Error::from_code(asContext_SetArgByte(self.context, arg, value)) }
    }

    pub fn set_arg_u16(&self, arg: u32, value: u16) -> Result<()> {
        unsafe { Error::from_code(asContext_SetArgWord(self.context, arg, value)) }
    }

    pub fn set_arg_u32(&self, arg: u32, value: u32) -> Result<()> {
        unsafe { Error::from_code(asContext_SetArgDWord(self.context, arg, value)) }
    }

    pub fn set_arg_u64(&self, arg: u32, value: u64) -> Result<()> {
        unsafe { Error::from_code(asContext_SetArgQWord(self.context, arg, value)) }
    }

    pub fn set_arg_float(&self, arg: u32, value: f32) -> Result<()> {
        unsafe { Error::from_code(asContext_SetArgFloat(self.context, arg, value)) }
    }

    pub fn set_arg_double(&self, arg: u32, value: f64) -> Result<()> {
        unsafe { Error::from_code(asContext_SetArgDouble(self.context, arg, value)) }
    }

    pub fn set_arg_str(&self, arg: u32, str: &str) -> Result<()> {
        let c_string = CString::new(str)?;
        unsafe {
            Error::from_code(asContext_SetArgObject(
                self.context,
                arg,
                c_string.into_raw() as *mut c_void,
            ))
        }
    }

    pub fn set_arg_object<T>(&self, arg: u32, obj: &mut T) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgObject(
                self.context,
                arg,
                obj as *mut _ as *mut c_void,
            ))
        }
    }

    // Return value
    pub fn get_return_byte(&self) -> u8 {
        unsafe { asContext_GetReturnByte(self.context) }
    }

    pub fn get_return_word(&self) -> u16 {
        unsafe { asContext_GetReturnWord(self.context) }
    }

    pub fn get_return_dword(&self) -> u32 {
        unsafe { asContext_GetReturnDWord(self.context) }
    }

    pub fn get_return_qword(&self) -> u64 {
        unsafe { asContext_GetReturnQWord(self.context) }
    }

    pub fn get_return_float(&self) -> f32 {
        unsafe { asContext_GetReturnFloat(self.context) }
    }

    pub fn get_return_double(&self) -> f64 {
        unsafe { asContext_GetReturnDouble(self.context) }
    }

    pub fn get_return_object<'a, T>(&self) -> &'a mut T {
        unsafe {
            let ptr = asContext_GetReturnObject(self.context);
            T::from_mut(ptr)
        }
    }

    // Exception handling
    pub fn set_exception(&self, string: &str) -> Result<()> {
        let c_string = CString::new(string)?;

        unsafe { Error::from_code(asContext_SetException(self.context, c_string.as_ptr())) }
    }

    pub fn get_exception_line_number(&self) -> (i32, Option<i32>, Option<&str>) {
        let mut column: i32 = 0;
        let mut section_name: *const c_char = ptr::null();

        unsafe {
            let line =
                asContext_GetExceptionLineNumber(self.context, &mut column, &mut section_name);

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
            let func = asContext_GetExceptionFunction(self.context);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_exception_string(&self) -> Option<&str> {
        unsafe {
            let string = asContext_GetExceptionString(self.context);
            if string.is_null() {
                None
            } else {
                CStr::from_ptr(string).to_str().ok()
            }
        }
    }

    // Debugging
    pub fn get_callstack_size(&self) -> u32 {
        unsafe { asContext_GetCallstackSize(self.context) }
    }

    pub fn get_function(&self, stack_level: u32) -> Option<Function> {
        unsafe {
            let func = asContext_GetFunction(self.context, stack_level);
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
            let line =
                asContext_GetLineNumber(self.context, stack_level, &mut column, &mut section_name);

            let section = if section_name.is_null() {
                None
            } else {
                CStr::from_ptr(section_name).to_str().ok()
            };

            (line, Some(column), section)
        }
    }

    // Variables
    pub fn get_var_count(&self, stack_level: u32) -> i32 {
        unsafe { asContext_GetVarCount(self.context, stack_level) }
    }

    pub fn get_var_declaration(
        &self,
        var_index: u32,
        stack_level: u32,
        include_namespace: bool,
    ) -> Option<&str> {
        unsafe {
            let decl = asContext_GetVarDeclaration(
                self.context,
                var_index,
                stack_level,
                as_bool(include_namespace),
            );
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    pub fn is_var_in_scope(&self, var_index: u32, stack_level: u32) -> bool {
        unsafe { from_as_bool(asContext_IsVarInScope(self.context, var_index, stack_level)) }
    }

    // This pointer
    pub fn get_this_type_id(&self, stack_level: u32) -> i32 {
        unsafe { asContext_GetThisTypeId(self.context, stack_level) }
    }

    pub fn get_this_pointer<'a, T>(&self, stack_level: u32) -> &'a mut T {
        unsafe {
            let ptr = asContext_GetThisPointer(self.context, stack_level);
            T::from_mut(ptr)
        }
    }

    // System function
    pub fn get_system_function(&self) -> Option<Function> {
        unsafe {
            let func = asContext_GetSystemFunction(self.context);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    // User data
    pub fn get_user_data<'a, T: UserData>(&self) -> Result<&'a mut T> {
        unsafe {
            let ptr = asContext_GetUserData(self.context, T::TypeId);
            if ptr.is_null() {
                return Err(Error::NullPointer)
            }
            Ok(T::from_mut(ptr))
        }
    }

    pub fn set_user_data<'a, T: UserData>(&self, data: &mut T) -> Option<&'a mut T> {
        unsafe {
            let ptr = asContext_SetUserData(self.context, data as *mut _ as *mut c_void, T::TypeId);
            if ptr.is_null() {
                return None
            }
            Some(T::from_mut(ptr))
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut asIScriptContext {
        self.context
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            asContext_Release(self.context);
        }
    }
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}
