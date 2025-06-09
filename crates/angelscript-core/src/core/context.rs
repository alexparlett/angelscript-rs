use crate::types::callbacks::{ExceptionCallbackFn, LineCallbackFn};
use angelscript_sys::*;
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

/// A script execution context.
///
/// The Context represents a single execution thread for AngelScript. It manages the call stack,
/// variable storage, and execution state for script functions. Multiple contexts can be created
/// from a single engine to allow concurrent script execution.
///
/// # Thread Safety
///
/// Each Context is designed to be used by a single thread. If you need to execute scripts
/// from multiple threads, create separate contexts for each thread.
///
/// # Examples
///
/// ```rust
/// use angelscript_core::core::engine::Engine;
/// use angelscript_core::types::enums::GetModuleFlags;
///
/// let engine = Engine::create()?;
/// let module = engine.get_module("MyModule", GetModuleFlags::CreateIfNotExists)?;
///
/// module.add_script_section("script", r#"
///     int add(int a, int b) {
///         return a + b;
///     }
/// "#, 0)?;
/// module.build()?;
///
/// let context = engine.create_context()?;
/// let function = module.get_function_by_name("add")?;
///
/// context.prepare(&function)?;
/// context.set_arg_dword(0, 5)?;
/// context.set_arg_dword(1, 3)?;
/// context.execute()?;
///
/// let result = context.get_return_dword();
/// assert_eq!(result, 8);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Context {
    context: *mut asIScriptContext,
}

impl Context {
    /// Creates a Context wrapper from a raw AngelScript context pointer.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid and the context is properly initialized.
    ///
    /// # Arguments
    /// * `context` - Raw pointer to AngelScript context
    ///
    /// # Returns
    /// A new Context wrapper
    pub(crate) fn from_raw(context: *mut asIScriptContext) -> Self {
        let wrapper = Context { context };
        wrapper
            .add_ref()
            .expect("Failed to add reference to context");
        wrapper
    }

    // Reference counting (matches vtable order)

    /// Increments the reference count of the context.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn add_ref(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_AddRef)(self.context)) }
    }

    /// Decrements the reference count of the context.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn release(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_Release)(self.context)) }
    }

    /// Gets the engine that created this context.
    ///
    /// # Returns
    /// The engine instance or an error
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptContext_GetEngine)(self.context);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    // Execution control

    /// Prepares the context for executing a function.
    ///
    /// This must be called before setting arguments and executing the function.
    ///
    /// # Arguments
    /// * `func` - The function to prepare for execution
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// let function = module.get_function_by_name("myFunction")?;
    /// context.prepare(&function)?;
    /// ```
    pub fn prepare(&self, func: &Function) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_Prepare)(
                self.context,
                func.as_raw(),
            ))
        }
    }

    /// Unprepares the context, cleaning up the current function call.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn unprepare(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_Unprepare)(self.context))
        }
    }

    /// Executes the prepared function.
    ///
    /// # Returns
    /// The execution state indicating how the execution completed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use angelscript_core::types::enums::ContextState;
    /// match context.execute()? {
    ///     ContextState::Finished => println!("Function completed successfully"),
    ///     ContextState::Suspended => println!("Function was suspended"),
    ///     ContextState::Aborted => println!("Function was aborted"),
    ///     ContextState::Exception => println!("Function threw an exception"),
    ///     _ => println!("Other execution state"),
    /// }
    /// ```
    pub fn execute(&self) -> ScriptResult<ContextState> {
        unsafe {
            let result = (self.as_vtable().asIScriptContext_Execute)(self.context);
            ScriptError::from_code(result)?;
            Ok(ContextState::from(result as u32))
        }
    }

    /// Aborts the execution of the current function.
    ///
    /// This can be called from another thread to stop a running script.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn abort(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_Abort)(self.context)) }
    }

    /// Suspends the execution of the current function.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn suspend(&self) -> ScriptResult<()> {
        unsafe { ScriptError::from_code((self.as_vtable().asIScriptContext_Suspend)(self.context)) }
    }

    /// Gets the current execution state of the context.
    ///
    /// # Returns
    /// The current context state
    pub fn get_state(&self) -> ContextState {
        unsafe { ContextState::from((self.as_vtable().asIScriptContext_GetState)(self.context)) }
    }

    // State management

    /// Pushes the current execution state onto the stack.
    ///
    /// This allows for nested function calls and context switching.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn push_state(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_PushState)(self.context))
        }
    }

    /// Pops the execution state from the stack.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn pop_state(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_PopState)(self.context))
        }
    }

    /// Checks if the context is nested and returns the nesting level.
    ///
    /// # Returns
    /// A tuple of (is_nested, nest_count)
    pub fn is_nested(&self) -> (bool, u32) {
        let mut nest_count: u32 = 0;
        unsafe {
            let is_nested =
                (self.as_vtable().asIScriptContext_IsNested)(self.context, &mut nest_count);
            (is_nested, nest_count)
        }
    }

    // Object context

    /// Sets the object instance for method calls.
    ///
    /// # Arguments
    /// * `obj` - The object instance to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_object<T: ScriptData>(&self, obj: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetObject)(
                self.context,
                obj.to_script_ptr(),
            ))
        }
    }

    // Arguments (in vtable order)

    /// Sets a byte argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `value` - The byte value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_byte(&self, arg: asUINT, value: asBYTE) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgByte)(
                self.context,
                arg,
                value,
            ))
        }
    }

    /// Sets a word (16-bit) argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `value` - The word value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_word(&self, arg: asUINT, value: asWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgWord)(
                self.context,
                arg,
                value,
            ))
        }
    }

    /// Sets a double word (32-bit) argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `value` - The dword value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Set integer arguments
    /// context.set_arg_dword(0, 42)?;
    /// context.set_arg_dword(1, 100)?;
    /// ```
    pub fn set_arg_dword(&self, arg: asUINT, value: asDWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgDWord)(
                self.context,
                arg,
                value,
            ))
        }
    }

    /// Sets a quad word (64-bit) argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `value` - The qword value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_qword(&self, arg: asUINT, value: asQWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgQWord)(
                self.context,
                arg,
                value,
            ))
        }
    }

    /// Sets a float argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `value` - The float value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_float(&self, arg: asUINT, value: f32) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgFloat)(
                self.context,
                arg,
                value,
            ))
        }
    }

    /// Sets a double argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `value` - The double value to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_double(&self, arg: asUINT, value: f64) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgDouble)(
                self.context,
                arg,
                value,
            ))
        }
    }

    /// Sets an address argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `addr` - The address to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_address<T: ScriptData>(&self, arg: asUINT, addr: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgAddress)(
                self.context,
                arg,
                addr.to_script_ptr(),
            ))
        }
    }

    /// Sets an object argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `obj` - The object to set
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn set_arg_object<T: ScriptData>(&self, arg: asUINT, obj: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_SetArgObject)(
                self.context,
                arg,
                obj.to_script_ptr(),
            ))
        }
    }

    /// Sets a variable type argument for the function call.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    /// * `ptr` - The pointer to the variable
    /// * `type_id` - The type ID of the variable
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Gets the address of an argument.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The address of the argument, or None if invalid
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

    /// Gets the byte return value from the function call.
    ///
    /// # Returns
    /// The byte return value
    pub fn get_return_byte(&self) -> asBYTE {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnByte)(self.context) }
    }

    /// Gets the word return value from the function call.
    ///
    /// # Returns
    /// The word return value
    pub fn get_return_word(&self) -> asWORD {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnWord)(self.context) }
    }

    /// Gets the double word return value from the function call.
    ///
    /// # Returns
    /// The dword return value
    ///
    /// # Examples
    ///
    /// ```rust
    /// context.execute()?;
    /// let result = context.get_return_dword();
    /// println!("Function returned: {}", result);
    /// ```
    pub fn get_return_dword(&self) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnDWord)(self.context) }
    }

    /// Gets the quad word return value from the function call.
    ///
    /// # Returns
    /// The qword return value
    pub fn get_return_qword(&self) -> asQWORD {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnQWord)(self.context) }
    }

    /// Gets the float return value from the function call.
    ///
    /// # Returns
    /// The float return value
    pub fn get_return_float(&self) -> f32 {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnFloat)(self.context) }
    }

    /// Gets the double return value from the function call.
    ///
    /// # Returns
    /// The double return value
    pub fn get_return_double(&self) -> f64 {
        unsafe { (self.as_vtable().asIScriptContext_GetReturnDouble)(self.context) }
    }

    /// Gets the address return value from the function call.
    ///
    /// # Returns
    /// The address return value, or None if null
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

    /// Gets the object return value from the function call.
    ///
    /// # Returns
    /// The object return value, or None if null
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

    /// Gets the address of the return value.
    ///
    /// # Returns
    /// The address of the return value, or None if not available
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

    /// Sets an exception in the context.
    ///
    /// # Arguments
    /// * `string` - The exception message
    /// * `allow_catch` - Whether the exception can be caught by script code
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Gets the line number where an exception occurred.
    ///
    /// # Returns
    /// A tuple of (line_number, column, section_name)
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

    /// Gets the function where an exception occurred.
    ///
    /// # Returns
    /// The function where the exception occurred, or None if not available
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

    /// Gets the exception message.
    ///
    /// # Returns
    /// The exception message, or None if no exception
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

    /// Checks if an exception will be caught by script code.
    ///
    /// # Returns
    /// true if the exception will be caught, false otherwise
    pub fn will_exception_be_caught(&self) -> bool {
        unsafe { (self.as_vtable().asIScriptContext_WillExceptionBeCaught)(self.context) }
    }

    /// Sets a callback for when exceptions occur.
    ///
    /// # Arguments
    /// * `callback` - The exception callback function
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Clears the exception callback.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn clear_exception_callback(&self) -> ScriptResult<()> {
        CallbackManager::set_exception_callback(None)?;
        unsafe {
            (self.as_vtable().asIScriptContext_ClearExceptionCallback)(self.context);
        }
        Ok(())
    }

    /// Sets a callback for line execution.
    ///
    /// This callback is called for each line of script code executed.
    ///
    /// # Arguments
    /// * `callback` - The line callback function
    /// * `param` - Parameter to pass to the callback
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Clears the line callback.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn clear_line_callback(&mut self) -> ScriptResult<()> {
        CallbackManager::set_line_callback(None)?;
        unsafe {
            (self.as_vtable().asIScriptContext_ClearLineCallback)(self.context);
        }
        Ok(())
    }

    // Call stack inspection

    /// Gets the size of the call stack.
    ///
    /// # Returns
    /// The number of functions in the call stack
    pub fn get_callstack_size(&self) -> u32 {
        unsafe { (self.as_vtable().asIScriptContext_GetCallstackSize)(self.context) }
    }

    /// Gets a function from the call stack.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level (0 is the current function)
    ///
    /// # Returns
    /// The function at the specified stack level, or None if invalid
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

    /// Gets the line number for a function in the call stack.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level (0 is the current function)
    ///
    /// # Returns
    /// A tuple of (line_number, column, section_name)
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

    /// Gets the number of variables at a stack level.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to inspect
    ///
    /// # Returns
    /// The number of variables
    pub fn get_var_count(&self, stack_level: u32) -> i32 {
        unsafe { (self.as_vtable().asIScriptContext_GetVarCount)(self.context, stack_level) }
    }

    /// Gets information about a variable.
    ///
    /// # Arguments
    /// * `var_index` - The variable index
    /// * `stack_level` - The stack level
    ///
    /// # Returns
    /// Variable information or an error
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

    /// Gets the declaration string for a variable.
    ///
    /// # Arguments
    /// * `var_index` - The variable index
    /// * `stack_level` - The stack level
    /// * `include_namespace` - Whether to include the namespace
    ///
    /// # Returns
    /// The variable declaration, or None if not available
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

    /// Gets the address of a variable.
    ///
    /// # Arguments
    /// * `var_index` - The variable index
    /// * `stack_level` - The stack level
    /// * `dont_dereference` - Whether to avoid dereferencing
    /// * `return_address_of_uninitialized_objects` - Whether to return addresses of uninitialized objects
    ///
    /// # Returns
    /// The address of the variable, or None if not available
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

    /// Checks if a variable is in scope.
    ///
    /// # Arguments
    /// * `var_index` - The variable index
    /// * `stack_level` - The stack level
    ///
    /// # Returns
    /// true if the variable is in scope, false otherwise
    pub fn is_var_in_scope(&self, var_index: u32, stack_level: u32) -> bool {
        unsafe {
            (self.as_vtable().asIScriptContext_IsVarInScope)(self.context, var_index, stack_level)
        }
    }

    // This pointer

    /// Gets the type ID of the 'this' pointer at a stack level.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level
    ///
    /// # Returns
    /// The type ID of the 'this' pointer
    pub fn get_this_type_id(&self, stack_level: u32) -> i32 {
        unsafe { (self.as_vtable().asIScriptContext_GetThisTypeId)(self.context, stack_level) }
    }

    /// Gets the 'this' pointer at a stack level.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level
    ///
    /// # Returns
    /// The 'this' pointer, or None if not available
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

    /// Gets the current system function being called.
    ///
    /// # Returns
    /// The system function, or None if not in a system function call
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

    /// Sets user data on the context.
    ///
    /// # Arguments
    /// * `data` - The user data to set
    ///
    /// # Returns
    /// The previous user data, or None if none was set
    pub fn set_user_data<T: UserData + ScriptData>(&self, data: &mut T) -> Option<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_SetUserData)(
                self.context,
                data.to_script_ptr(),
                T::KEY,
            );
            if ptr.is_null() {
                None
            } else {
                Some(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    /// Gets user data from the context.
    ///
    /// # Returns
    /// The user data or an error if not found
    pub fn get_user_data<T: UserData + ScriptData>(&self) -> ScriptResult<T> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptContext_GetUserData)(self.context, T::KEY);
            if ptr.is_null() {
                Err(ScriptError::NullPointer)
            } else {
                Ok(ScriptData::from_script_ptr(ptr))
            }
        }
    }

    // Serialization

    /// Starts deserialization of the context state.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn start_deserialization(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_StartDeserialization)(
                self.context,
            ))
        }
    }

    /// Finishes deserialization of the context state.
    ///
    /// # Returns
    /// Result indicating success or failure
    pub fn finish_deserialization(&self) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptContext_FinishDeserialization)(
                self.context,
            ))
        }
    }

    /// Pushes a function onto the call stack.
    ///
    /// # Arguments
    /// * `func` - The function to push
    /// * `object` - Optional object instance for method calls
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Gets the state registers for debugging purposes.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to inspect
    ///
    /// # Returns
    /// State register information or an error
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

    /// Gets the call state registers for debugging purposes.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to inspect
    ///
    /// # Returns
    /// Call state register information or an error
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

    /// Sets the state registers for debugging purposes.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to modify
    /// * `calling_system_function` - The calling system function
    /// * `initial_function` - The initial function
    /// * `orig_stack_pointer` - Original stack pointer
    /// * `arguments_size` - Size of arguments
    /// * `value_register` - Value register
    /// * `object_register` - Object register
    /// * `object_type_register` - Object type register
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Sets the call state registers for debugging purposes.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to modify
    /// * `stack_frame_pointer` - Stack frame pointer
    /// * `current_function` - Current function
    /// * `program_pointer` - Program pointer
    /// * `stack_pointer` - Stack pointer
    /// * `stack_index` - Stack index
    ///
    /// # Returns
    /// Result indicating success or failure
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

    /// Gets the number of arguments on the stack.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to inspect
    ///
    /// # Returns
    /// The number of arguments or an error
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

    /// Gets information about an argument on the stack.
    ///
    /// # Arguments
    /// * `stack_level` - The stack level to inspect
    /// * `arg` - The argument index
    ///
    /// # Returns
    /// Stack argument information or an error
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

    /// Gets the raw context pointer for internal use.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript context pointer.
    pub(crate) fn as_ptr(&self) -> *mut asIScriptContext {
        self.context
    }

    /// Gets the vtable for the underlying AngelScript context.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
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

/// Information about a variable in the script context.
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// The variable name
    pub name: Option<String>,
    /// The type ID of the variable
    pub type_id: i32,
    /// Type modifiers (const, reference, etc.)
    pub type_modifiers: TypeModifiers,
    /// Whether the variable is allocated on the heap
    pub is_var_on_heap: bool,
    /// Offset of the variable on the stack
    pub stack_offset: i32,
}

/// State registers for debugging purposes.
#[derive(Debug)]
pub struct StateRegisters {
    /// The calling system function
    pub calling_system_function: Option<Function>,
    /// The initial function
    pub initial_function: Option<Function>,
    /// Original stack pointer
    pub orig_stack_pointer: asDWORD,
    /// Size of arguments
    pub arguments_size: asDWORD,
    /// Value register
    pub value_register: asQWORD,
    /// Object register
    pub object_register: Option<ScriptMemoryLocation>,
    /// Object type register
    pub object_type_register: Option<TypeInfo>,
}

/// Call state registers for debugging purposes.
#[derive(Debug)]
pub struct CallStateRegisters {
    /// Stack frame pointer
    pub stack_frame_pointer: asDWORD,
    /// Current function
    pub current_function: Option<Function>,
    /// Program pointer
    pub program_pointer: asDWORD,
    /// Stack pointer
    pub stack_pointer: asDWORD,
    /// Stack index
    pub stack_index: asDWORD,
}

/// Information about an argument on the stack.
#[derive(Debug)]
pub struct StackArgument<T> {
    /// The type ID of the argument
    pub type_id: i32,
    /// Flags for the argument
    pub flags: asUINT,
    /// Address of the argument
    pub address: Option<T>,
}
