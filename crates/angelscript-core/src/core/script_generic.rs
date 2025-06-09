use crate::core::engine::Engine;
use crate::core::error::{ScriptError, ScriptResult};
use crate::core::function::Function;
use crate::types::enums::{TypeId, TypeModifiers};
use crate::types::script_data::ScriptData;
use crate::types::script_memory::ScriptMemoryLocation;
use angelscript_sys::*;
use std::ptr::NonNull;

/// A wrapper for AngelScript's generic interface.
///
/// The `ScriptGeneric` interface is used when calling registered functions with the
/// generic calling convention. It provides a unified way to access function arguments,
/// return values, and context information regardless of the specific parameter types.
///
/// # Generic Calling Convention
///
/// The generic calling convention is AngelScript's most flexible but slowest calling
/// convention. It's used when:
/// - The native calling convention cannot be determined
/// - Maximum portability is required
/// - Complex parameter types need special handling
/// - Functions are registered with `CallingConvention::Generic`
///
/// # Function Registration
///
/// When registering functions with the generic calling convention, your function
/// signature should be:
///
/// ```rust
/// fn my_function(gen: &ScriptGeneric) {
///     // Access arguments and set return values through gen
/// }
/// ```
///
/// # Examples
///
/// ## Basic Function Registration
///
/// ```rust
/// use angelscript_rs::{Engine, ScriptGeneric};
///
/// fn add_function(gen: &ScriptGeneric) {
///     // Get arguments
///     let a: i32 = gen.get_arg_typed(0).unwrap_or(0);
///     let b: i32 = gen.get_arg_typed(1).unwrap_or(0);
///
///     // Calculate result
///     let result = a + b;
///
///     // Set return value
///     gen.set_return_dword(result as u32).expect("Failed to set return value");
/// }
///
/// let engine = Engine::create()?;
/// engine.register_global_function(
///     "int add(int a, int b)",
///     add_function,
///     None
/// )?;
/// ```
///
/// ## Object Method Registration
///
/// ```rust
/// fn object_method(gen: &ScriptGeneric) {
///     // Get the object instance
///     if let Some(obj) = gen.get_object() {
///         // Cast to your object type
///         // let my_obj = unsafe { &mut *(obj.as_mut_ptr() as *mut MyObject) };
///
///         // Get method arguments
///         let arg: i32 = gen.get_arg_typed(0).unwrap_or(0);
///
///         // Perform operation and set return value
///         // let result = my_obj.some_method(arg);
///         // gen.set_return_dword(result).expect("Failed to set return");
///     }
/// }
/// ```
///
/// ## Complex Type Handling
///
/// ```rust
/// fn string_function(gen: &ScriptGeneric) {
///     // Get string argument by address
///     if let Some(str_addr) = gen.get_arg_address(0) {
///         // Handle string type according to your string implementation
///         // let script_string = unsafe { &*(str_addr.as_ptr() as *const ScriptString) };
///
///         // Process the string and return result
///         // let result = process_string(script_string);
///         // gen.set_return_object(&mut result).expect("Failed to set return");
///     }
/// }
/// ```
///
/// ## Error Handling
///
/// ```rust
/// fn safe_divide(gen: &ScriptGeneric) {
///     let a: f64 = gen.get_arg_typed(0).unwrap_or(0.0);
///     let b: f64 = gen.get_arg_typed(1).unwrap_or(1.0);
///
///     if b == 0.0 {
///         // Set exception in the script context
///         if let Ok(engine) = gen.get_engine() {
///             if let Some(ctx) = Engine::get_active_context() {
///                 ctx.set_exception("Division by zero", true)
///                     .expect("Failed to set exception");
///                 return;
///             }
///         }
///     }
///
///     let result = a / b;
///     gen.set_return_double(result).expect("Failed to set return value");
/// }
/// ```
#[derive(Debug)]
pub struct ScriptGeneric {
    inner: *mut asIScriptGeneric,
}

impl ScriptGeneric {
    /// Creates a ScriptGeneric wrapper from a raw AngelScript pointer.
    ///
    /// # Safety
    /// The pointer must be valid and point to a properly initialized `asIScriptGeneric`.
    /// This function is typically called by the AngelScript engine when invoking
    /// generic functions.
    ///
    /// # Arguments
    /// * `ptr` - Raw pointer to AngelScript generic interface
    ///
    /// # Returns
    /// A new ScriptGeneric wrapper
    pub(crate) fn from_raw(ptr: *mut asIScriptGeneric) -> Self {
        Self { inner: ptr }
    }

    // ========== VTABLE ORDER (matches asIScriptGeneric__bindgen_vtable) ==========

    /// Gets the engine that owns this generic interface.
    ///
    /// # Returns
    /// The engine instance or an error if the engine is not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn my_function(gen: &ScriptGeneric) {
    ///     let engine = gen.get_engine().expect("Failed to get engine");
    ///     println!("Engine version: {}", Engine::get_library_version());
    /// }
    /// ```
    pub fn get_engine(&self) -> ScriptResult<Engine> {
        unsafe {
            let result: *mut asIScriptEngine =
                (self.as_vtable().asIScriptGeneric_GetEngine)(self.inner);
            let ptr = NonNull::new(result).ok_or(ScriptError::NullPointer)?;
            Ok(Engine::from_raw(ptr))
        }
    }

    /// Gets the function being called.
    ///
    /// This provides access to metadata about the function, such as its name,
    /// declaration, and parameter information.
    ///
    /// # Returns
    /// The function being called
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn my_function(gen: &ScriptGeneric) {
    ///     let function = gen.get_function();
    ///     if let Some(name) = function.get_name() {
    ///         println!("Called function: {}", name);
    ///     }
    /// }
    /// ```
    pub fn get_function(&self) -> Function {
        unsafe { Function::from_raw((self.as_vtable().asIScriptGeneric_GetFunction)(self.inner)) }
    }

    /// Gets auxiliary data associated with the function.
    ///
    /// Auxiliary data is custom data that was provided when the function was registered.
    /// This can be used to store additional context or configuration for the function.
    ///
    /// # Returns
    /// The auxiliary data
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn my_function(gen: &ScriptGeneric) {
    ///     // Assuming auxiliary data was registered as MyAuxData
    ///     let aux_data: MyAuxData = gen.get_auxiliary();
    ///     // Use auxiliary data for function behavior
    /// }
    /// ```
    pub fn get_auxiliary<T: ScriptData>(&self) -> T {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetAuxiliary)(self.inner);
            ScriptData::from_script_ptr(ptr)
        }
    }

    /// Gets the object instance for method calls.
    ///
    /// For object methods, this returns a pointer to the object instance that
    /// the method is being called on. For global functions, this returns None.
    ///
    /// # Returns
    /// The object instance, or None for global functions
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn object_method(gen: &ScriptGeneric) {
    ///     if let Some(obj) = gen.get_object() {
    ///         // This is a method call - obj points to the instance
    ///         // Cast to your specific object type
    ///         // let my_obj = unsafe { &mut *(obj.as_mut_ptr() as *mut MyObject) };
    ///         println!("Method called on object");
    ///     } else {
    ///         println!("Global function called");
    ///     }
    /// }
    /// ```
    pub fn get_object(&self) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetObject)(self.inner);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    /// Gets the type ID of the object instance.
    ///
    /// For object methods, this returns the type ID of the object that the method
    /// is being called on. This can be used for type checking or casting.
    ///
    /// # Returns
    /// The object's type ID, or 0 for global functions
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn object_method(gen: &ScriptGeneric) {
    ///     let obj_type_id = gen.get_object_type_id();
    ///     if obj_type_id != 0 {
    ///         println!("Method called on object with type ID: {}", obj_type_id);
    ///     }
    /// }
    /// ```
    pub fn get_object_type_id(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetObjectTypeId)(self.inner) }
    }

    /// Gets the number of arguments passed to the function.
    ///
    /// # Returns
    /// The number of arguments
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn variadic_function(gen: &ScriptGeneric) {
    ///     let arg_count = gen.get_arg_count();
    ///     println!("Function called with {} arguments", arg_count);
    ///
    ///     for i in 0..arg_count as u32 {
    ///         let (type_id, flags) = gen.get_arg_type_id(i);
    ///         println!("Argument {}: type_id={:?}, flags={:?}", i, type_id, flags);
    ///     }
    /// }
    /// ```
    pub fn get_arg_count(&self) -> i32 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgCount)(self.inner) }
    }

    /// Gets the type ID and flags for a specific argument.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// A tuple of (type_id, type_modifiers)
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn inspect_args(gen: &ScriptGeneric) {
    ///     for i in 0..gen.get_arg_count() as u32 {
    ///         let (type_id, flags) = gen.get_arg_type_id(i);
    ///
    ///         match type_id {
    ///             TypeId::Int32 => println!("Argument {} is int32", i),
    ///             TypeId::Float => println!("Argument {} is float", i),
    ///             _ => println!("Argument {} has type ID: {:?}", i, type_id),
    ///         }
    ///
    ///         if flags.contains(TypeModifiers::CONST) {
    ///             println!("Argument {} is const", i);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn get_arg_type_id(&self, arg: asUINT) -> (TypeId, TypeModifiers) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id = TypeId::from((self.as_vtable().asIScriptGeneric_GetArgTypeId)(
                self.inner, arg, &mut flags,
            ) as u32);
            let typed_id_flags = TypeModifiers::from(flags);
            (type_id, typed_id_flags)
        }
    }

    /// Gets a byte argument value.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The byte value
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn byte_function(gen: &ScriptGeneric) {
    ///     let byte_val = gen.get_arg_byte(0);
    ///     println!("Received byte: {}", byte_val);
    /// }
    /// ```
    pub fn get_arg_byte(&self, arg: asUINT) -> asBYTE {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgByte)(self.inner, arg) }
    }

    /// Gets a word (16-bit) argument value.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The word value
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn word_function(gen: &ScriptGeneric) {
    ///     let word_val = gen.get_arg_word(0);
    ///     println!("Received word: {}", word_val);
    /// }
    /// ```
    pub fn get_arg_word(&self, arg: asUINT) -> asWORD {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgWord)(self.inner, arg) }
    }

    /// Gets a double word (32-bit) argument value.
    ///
    /// This is commonly used for 32-bit integers and single-precision floats.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The dword value
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn int_function(gen: &ScriptGeneric) {
    ///     let int_val = gen.get_arg_dword(0) as i32;
    ///     println!("Received int: {}", int_val);
    ///
    ///     // Return the value doubled
    ///     gen.set_return_dword((int_val * 2) as u32)
    ///         .expect("Failed to set return value");
    /// }
    /// ```
    pub fn get_arg_dword(&self, arg: asUINT) -> asDWORD {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgDWord)(self.inner, arg) }
    }

    /// Gets a quad word (64-bit) argument value.
    ///
    /// This is commonly used for 64-bit integers and double-precision floats.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The qword value
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn long_function(gen: &ScriptGeneric) {
    ///     let long_val = gen.get_arg_qword(0) as i64;
    ///     println!("Received long: {}", long_val);
    /// }
    /// ```
    pub fn get_arg_qword(&self, arg: asUINT) -> asQWORD {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgQWord)(self.inner, arg) }
    }

    /// Gets a float argument value.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The float value
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn float_function(gen: &ScriptGeneric) {
    ///     let float_val = gen.get_arg_float(0);
    ///     println!("Received float: {}", float_val);
    ///
    ///     // Return the square root
    ///     gen.set_return_float(float_val.sqrt())
    ///         .expect("Failed to set return value");
    /// }
    /// ```
    pub fn get_arg_float(&self, arg: asUINT) -> f32 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgFloat)(self.inner, arg) }
    }

    /// Gets a double argument value.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The double value
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn double_function(gen: &ScriptGeneric) {
    ///     let double_val = gen.get_arg_double(0);
    ///     println!("Received double: {}", double_val);
    /// }
    /// ```
    pub fn get_arg_double(&self, arg: asUINT) -> f64 {
        unsafe { (self.as_vtable().asIScriptGeneric_GetArgDouble)(self.inner, arg) }
    }

    /// Gets the address of an argument.
    ///
    /// This is used for reference parameters and complex types that are passed by address.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The argument address, or None if null
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn reference_function(gen: &ScriptGeneric) {
    ///     if let Some(addr) = gen.get_arg_address(0) {
    ///         // For an int& parameter
    ///         let int_ref = unsafe { &mut *(addr.as_mut_ptr() as *mut i32) };
    ///         *int_ref += 10; // Modify the referenced value
    ///     }
    /// }
    /// ```
    pub fn get_arg_address(&self, arg: asUINT) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetArgAddress)(self.inner, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    /// Gets an object argument.
    ///
    /// This is used for complex types that are passed by value or handle.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The object pointer, or None if null
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn object_function(gen: &ScriptGeneric) {
    ///     if let Some(obj) = gen.get_arg_object(0) {
    ///         // Cast to your specific object type
    ///         // let my_obj = unsafe { &*(obj.as_ptr() as *const MyObject) };
    ///         // Use the object...
    ///     }
    /// }
    /// ```
    pub fn get_arg_object(&self, arg: asUINT) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetArgObject)(self.inner, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    /// Gets the address of an argument's storage location.
    ///
    /// This provides direct access to where the argument is stored, which can be
    /// useful for modifying value types passed by reference.
    ///
    /// # Arguments
    /// * `arg` - The argument index (0-based)
    ///
    /// # Returns
    /// The storage address, or None if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn modify_arg(gen: &ScriptGeneric) {
    ///     if let Some(addr) = gen.get_address_of_arg(0) {
    ///         // Directly modify the argument's storage
    ///         let value = unsafe { &mut *(addr.as_mut_ptr() as *mut i32) };
    ///         *value *= 2;
    ///     }
    /// }
    /// ```
    pub fn get_address_of_arg(&self, arg: asUINT) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetAddressOfArg)(self.inner, arg);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    /// Gets the return type ID and flags.
    ///
    /// # Returns
    /// A tuple of (type_id, type_modifiers)
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn check_return_type(gen: &ScriptGeneric) {
    ///     let (return_type, flags) = gen.get_return_type_id();
    ///
    ///     match return_type {
    ///         TypeId::Void => {
    ///             // No return value needed
    ///         }
    ///         TypeId::Int32 => {
    ///             gen.set_return_dword(42).expect("Failed to set return");
    ///         }
    ///         TypeId::Float => {
    ///             gen.set_return_float(3.14).expect("Failed to set return");
    ///         }
    ///         _ => {
    ///             // Handle other types...
    ///         }
    ///     }
    /// }
    /// ```
    pub fn get_return_type_id(&self) -> (TypeId, TypeModifiers) {
        let mut flags: asDWORD = 0;
        unsafe {
            let type_id =
                (self.as_vtable().asIScriptGeneric_GetReturnTypeId)(self.inner, &mut flags);
            (TypeId::from(type_id as u32), TypeModifiers::from(flags))
        }
    }

    /// Sets a byte return value.
    ///
    /// # Arguments
    /// * `val` - The byte value to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_byte(gen: &ScriptGeneric) {
    ///     gen.set_return_byte(255).expect("Failed to set return value");
    /// }
    /// ```
    pub fn set_return_byte(&self, val: asBYTE) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnByte)(
                self.inner, val,
            ))
        }
    }

    /// Sets a word (16-bit) return value.
    ///
    /// # Arguments
    /// * `val` - The word value to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_word(gen: &ScriptGeneric) {
    ///     gen.set_return_word(65535).expect("Failed to set return value");
    /// }
    /// ```
    pub fn set_return_word(&self, val: asWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnWord)(
                self.inner, val,
            ))
        }
    }

    /// Sets a double word (32-bit) return value.
    ///
    /// This is commonly used for 32-bit integers and single-precision floats.
    ///
    /// # Arguments
    /// * `val` - The dword value to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_int(gen: &ScriptGeneric) {
    ///     let result = 42i32;
    ///     gen.set_return_dword(result as u32).expect("Failed to set return value");
    /// }
    /// ```
    pub fn set_return_dword(&self, val: asDWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnDWord)(
                self.inner, val,
            ))
        }
    }

    /// Sets a quad word (64-bit) return value.
    ///
    /// This is commonly used for 64-bit integers and double-precision floats.
    ///
    /// # Arguments
    /// * `val` - The qword value to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_long(gen: &ScriptGeneric) {
    ///     let result = 9223372036854775807i64;
    ///     gen.set_return_qword(result as u64).expect("Failed to set return value");
    /// }
    /// ```
    pub fn set_return_qword(&self, val: asQWORD) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnQWord)(
                self.inner, val,
            ))
        }
    }

    /// Sets a float return value.
    ///
    /// # Arguments
    /// * `val` - The float value to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_float(gen: &ScriptGeneric) {
    ///     gen.set_return_float(3.14159).expect("Failed to set return value");
    /// }
    /// ```
    pub fn set_return_float(&self, val: f32) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnFloat)(
                self.inner, val,
            ))
        }
    }

    /// Sets a double return value.
    ///
    /// # Arguments
    /// * `val` - The double value to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_double(gen: &ScriptGeneric) {
    ///     gen.set_return_double(2.718281828459045).expect("Failed to set return value");
    /// }
    /// ```
    pub fn set_return_double(&self, val: f64) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnDouble)(
                self.inner, val,
            ))
        }
    }

    /// Sets a return address using a raw memory location.
    ///
    /// # Arguments
    /// * `addr` - The memory location to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_address_raw(gen: &ScriptGeneric) {
    ///     let memory_loc = ScriptMemoryLocation::from_mut(some_ptr);
    ///     gen.set_return_address_raw(memory_loc).expect("Failed to set return address");
    /// }
    /// ```
    pub fn set_return_address_raw(&self, mut addr: ScriptMemoryLocation) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnAddress)(
                self.inner,
                addr.as_mut_ptr(),
            ))
        }
    }

    /// Sets a return address using a typed reference.
    ///
    /// # Arguments
    /// * `addr` - The typed reference to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_reference(gen: &ScriptGeneric) {
    ///     let mut value = 42i32;
    ///     gen.set_return_address(&mut value).expect("Failed to set return address");
    /// }
    /// ```
    pub fn set_return_address<T: ScriptData>(&self, addr: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnAddress)(
                self.inner,
                addr.to_script_ptr(),
            ))
        }
    }

    /// Sets an object return value.
    ///
    /// This is used for returning complex types by value or handle.
    ///
    /// # Arguments
    /// * `obj` - The object to return
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn return_object(gen: &ScriptGeneric) {
    ///     let mut my_object = MyObject::new();
    ///     gen.set_return_object(&mut my_object).expect("Failed to set return object");
    /// }
    /// ```
    pub fn set_return_object<T: ScriptData>(&self, obj: &mut T) -> ScriptResult<()> {
        unsafe {
            ScriptError::from_code((self.as_vtable().asIScriptGeneric_SetReturnObject)(
                self.inner,
                obj.to_script_ptr(),
            ))
        }
    }

    /// Gets the address where the return value should be stored.
    ///
    /// This is useful for constructing return values directly in their final location,
    /// avoiding unnecessary copies.
    ///
    /// # Returns
    /// The return value storage location, or None if not available
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn construct_return_value(gen: &ScriptGeneric) {
    ///     if let Some(return_addr) = gen.get_address_of_return_location() {
    ///         // Construct the return value directly in place
    ///         // let return_obj = unsafe { &mut *(return_addr.as_mut_ptr() as *mut MyObject) };
    ///         // *return_obj = MyObject::new();
    ///     }
    /// }
    /// ```
    pub fn get_address_of_return_location(&self) -> Option<ScriptMemoryLocation> {
        unsafe {
            let ptr = (self.as_vtable().asIScriptGeneric_GetAddressOfReturnLocation)(self.inner);
            if ptr.is_null() {
                None
            } else {
                Some(ScriptMemoryLocation::from_mut(ptr))
            }
        }
    }

    /// Gets all arguments as a vector of ScriptArg.
    ///
    /// This is a convenience method that collects all function arguments with their
    /// type information into a vector for easier processing.
    ///
    /// # Returns
    /// A vector containing all arguments with their type information
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn process_all_args(gen: &ScriptGeneric) {
    ///     let args = gen.get_all_args();
    ///
    ///     for (i, arg) in args.iter().enumerate() {
    ///         println!("Argument {}: type={:?}, flags={:?}",
    ///                  i, arg.type_id, arg.flags);
    ///
    ///         match &arg.value {
    ///             ScriptValue::Int32(val) => println!("  Value: {}", val),
    ///             ScriptValue::Float(val) => println!("  Value: {}", val),
    ///             ScriptValue::Double(val) => println!("  Value: {}", val),
    ///             _ => println!("  Complex value"),
    ///         }
    ///     }
    /// }
    /// ```
    pub fn get_all_args(&self) -> Vec<ScriptArg> {
        let count = self.get_arg_count();
        (0..count as asUINT)
            .map(|i| {
                let (type_id, flags) = self.get_arg_type_id(i);
                ScriptArg {
                    type_id,
                    flags,
                    value: self.get_address_of_arg(i),
                }
            })
            .collect()
    }

    /// Checks if the function has a return value.
    ///
    /// # Returns
    /// true if the function returns a value, false if it returns void
    ///
    /// # Examples
    ///
    /// ```rust
    /// fn conditional_return(gen: &ScriptGeneric) {
    ///     if gen.has_return_value() {
    ///         // Function expects a return value
    ///         gen.set_return_dword(42).expect("Failed to set return value");
    ///     } else {
    ///         // Void function - no return value needed
    ///         println!("Void function executed");
    ///     }
    /// }
    /// ```
    pub fn has_return_value(&self) -> bool {
        let (type_id, _) = self.get_return_type_id();
        type_id != TypeId::Void
    }

    /// Gets the vtable for the underlying AngelScript generic interface.
    ///
    /// # Safety
    /// This function provides direct access to the AngelScript vtable.
    fn as_vtable(&self) -> &asIScriptGeneric__bindgen_vtable {
        unsafe { &*(*self.inner).vtable_ }
    }
}

// ScriptGeneric doesn't manage its own lifetime - AngelScript does
unsafe impl Send for ScriptGeneric {}
unsafe impl Sync for ScriptGeneric {}

/// Represents a generic value with type information
#[derive(Debug, Clone)]
pub struct ScriptArg {
    pub type_id: TypeId,
    pub flags: TypeModifiers,
    pub value: Option<ScriptMemoryLocation>,
}
