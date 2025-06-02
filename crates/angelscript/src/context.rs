use crate::error::{Result, Error};
use crate::function::Function;
use crate::types::*;
use crate::enums::*;
use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::marker::PhantomData;

pub struct Context {
    context: *mut asIScriptContext,
    _phantom: PhantomData<asIScriptContext>,
}

impl Context {
    pub(crate) fn from_raw(context: *mut asIScriptContext) -> Self {
        Context {
            context,
            _phantom: PhantomData,
        }
    }

    pub fn get_engine(&self) -> *mut asIScriptEngine {
        unsafe {
            asContext_GetEngine(self.context)
        }
    }

    pub fn add_ref(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_AddRef(self.context))
        }
    }

    pub fn release(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_Release(self.context))
        }
    }

    // Execution
    pub fn get_state(&self) -> ContextState {
        unsafe {
            asContext_GetState(self.context)
        }
    }

    pub fn prepare(&self, func: &Function) -> Result<()> {
        unsafe {
            Error::from_code(asContext_Prepare(self.context, func.as_ptr()))
        }
    }

    pub fn unprepare(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_Unprepare(self.context))
        }
    }

    pub fn execute(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_Execute(self.context))
        }
    }

    pub fn abort(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_Abort(self.context))
        }
    }

    pub fn suspend(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_Suspend(self.context))
        }
    }

    // State management
    pub fn push_state(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_PushState(self.context))
        }
    }

    pub fn pop_state(&self) -> Result<()> {
        unsafe {
            Error::from_code(asContext_PopState(self.context))
        }
    }

    pub fn is_nested(&self) -> (bool, asUINT) {
        let mut nest_count: asUINT = 0;
        unsafe {
            let is_nested = asContext_IsNested(self.context, &mut nest_count);
            (from_as_bool(is_nested), nest_count)
        }
    }

    // Object pointer for calling class methods
    pub fn set_object(&self, obj: *mut c_void) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetObject(self.context, obj))
        }
    }

    // Arguments
    pub fn set_arg_byte(&self, arg: asUINT, value: asBYTE) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgByte(self.context, arg, value))
        }
    }

    pub fn set_arg_word(&self, arg: asUINT, value: asWORD) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgWord(self.context, arg, value))
        }
    }

    pub fn set_arg_dword(&self, arg: asUINT, value: asDWORD) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgDWord(self.context, arg, value))
        }
    }

    pub fn set_arg_qword(&self, arg: asUINT, value: asQWORD) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgQWord(self.context, arg, value))
        }
    }

    pub fn set_arg_float(&self, arg: asUINT, value: f32) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgFloat(self.context, arg, value))
        }
    }

    pub fn set_arg_double(&self, arg: asUINT, value: f64) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgDouble(self.context, arg, value))
        }
    }

    pub fn set_arg_address(&self, arg: asUINT, addr: *mut c_void) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgAddress(self.context, arg, addr))
        }
    }

    pub fn set_arg_object(&self, arg: asUINT, obj: *mut c_void) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgObject(self.context, arg, obj))
        }
    }

    pub fn set_arg_var_type(&self, arg: asUINT, ptr: *mut c_void, type_id: i32) -> Result<()> {
        unsafe {
            Error::from_code(asContext_SetArgVarType(self.context, arg, ptr, type_id))
        }
    }

    pub fn get_address_of_arg(&self, arg: asUINT) -> *mut c_void {
        unsafe {
            asContext_GetAddressOfArg(self.context, arg)
        }
    }

    // Return value
    pub fn get_return_byte(&self) -> asBYTE {
        unsafe {
            asContext_GetReturnByte(self.context)
        }
    }

    pub fn get_return_word(&self) -> asWORD {
        unsafe {
            asContext_GetReturnWord(self.context)
        }
    }

    pub fn get_return_dword(&self) -> asDWORD {
        unsafe {
            asContext_GetReturnDWord(self.context)
        }
    }

    pub fn get_return_qword(&self) -> asQWORD {
        unsafe {
            asContext_GetReturnQWord(self.context)
        }
    }

    pub fn get_return_float(&self) -> f32 {
        unsafe {
            asContext_GetReturnFloat(self.context)
        }
    }

    pub fn get_return_double(&self) -> f64 {
        unsafe {
            asContext_GetReturnDouble(self.context)
        }
    }

    pub fn get_return_address(&self) -> *mut c_void {
        unsafe {
            asContext_GetReturnAddress(self.context)
        }
    }

    pub fn get_return_object(&self) -> *mut c_void {
        unsafe {
            asContext_GetReturnObject(self.context)
        }
    }

    pub fn get_address_of_return_value(&self) -> *mut c_void {
        unsafe {
            asContext_GetAddressOfReturnValue(self.context)
        }
    }

    // Exception handling
    pub fn set_exception(&self, string: &str) -> Result<()> {
        let c_string = CString::new(string)?;

        unsafe {
            Error::from_code(asContext_SetException(self.context, c_string.as_ptr()))
        }
    }

    pub fn get_exception_line_number(&self) -> (i32, Option<i32>, Option<&str>) {
        let mut column: i32 = 0;
        let mut section_name: *const c_char = ptr::null();

        unsafe {
            let line = asContext_GetExceptionLineNumber(self.context, &mut column, &mut section_name);

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
    pub fn get_callstack_size(&self) -> asUINT {
        unsafe {
            asContext_GetCallstackSize(self.context)
        }
    }

    pub fn get_function(&self, stack_level: asUINT) -> Option<Function> {
        unsafe {
            let func = asContext_GetFunction(self.context, stack_level);
            if func.is_null() {
                None
            } else {
                Some(Function::from_raw(func))
            }
        }
    }

    pub fn get_line_number(&self, stack_level: asUINT) -> (i32, Option<i32>, Option<&str>) {
        let mut column: i32 = 0;
        let mut section_name: *const c_char = ptr::null();

        unsafe {
            let line = asContext_GetLineNumber(self.context, stack_level, &mut column, &mut section_name);

            let section = if section_name.is_null() {
                None
            } else {
                CStr::from_ptr(section_name).to_str().ok()
            };

            (line, Some(column), section)
        }
    }

    // Variables
    pub fn get_var_count(&self, stack_level: asUINT) -> i32 {
        unsafe {
            asContext_GetVarCount(self.context, stack_level)
        }
    }

    pub fn get_var_declaration(&self, var_index: asUINT, stack_level: asUINT, include_namespace: bool) -> Option<&str> {
        unsafe {
            let decl = asContext_GetVarDeclaration(self.context, var_index, stack_level, as_bool(include_namespace));
            if decl.is_null() {
                None
            } else {
                CStr::from_ptr(decl).to_str().ok()
            }
        }
    }

    pub fn get_address_of_var(&self, var_index: asUINT, stack_level: asUINT) -> *mut c_void {
        unsafe {
            asContext_GetAddressOfVar(self.context, var_index, stack_level)
        }
    }

    pub fn is_var_in_scope(&self, var_index: asUINT, stack_level: asUINT) -> bool {
        unsafe {
            from_as_bool(asContext_IsVarInScope(self.context, var_index, stack_level))
        }
    }

    // This pointer
    pub fn get_this_type_id(&self, stack_level: asUINT) -> i32 {
        unsafe {
            asContext_GetThisTypeId(self.context, stack_level)
        }
    }

    pub fn get_this_pointer(&self, stack_level: asUINT) -> *mut c_void {
        unsafe {
            asContext_GetThisPointer(self.context, stack_level)
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
    pub fn get_user_data(&self, type_: asPWORD) -> *mut c_void {
        unsafe {
            asContext_GetUserData(self.context, type_)
        }
    }

    pub fn set_user_data(&self, data: *mut c_void, type_: asPWORD) -> *mut c_void {
        unsafe {
            asContext_SetUserData(self.context, data, type_)
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
